use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::models::profile::SshProfile;
use crate::services::profile_service;
use crate::state::AppState;

// ── Fingerprint cache (avoid re-spawning ssh-keygen for the same key) ──

static FINGERPRINT_CACHE: std::sync::LazyLock<Mutex<HashMap<String, KeyFingerprint>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Export all profiles as JSON string
#[tauri::command]
pub fn export_profiles(state: State<'_, AppState>) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    let json = serde_json::to_string_pretty(&inner.profiles)?;
    Ok(json)
}

/// Import profiles from JSON string (merges, skips duplicates by name)
#[tauri::command]
pub fn import_profiles(
    json: String,
    state: State<'_, AppState>,
) -> Result<u32, MazeSshError> {
    ensure_unlocked(&state)?;

    let imported: Vec<SshProfile> = serde_json::from_str(&json)?;
    let mut inner = state.inner.write().map_err(|_| MazeSshError::StateLockError)?;
    let mut count = 0u32;

    for mut profile in imported {
        if inner.profiles.iter().any(|p| p.name == profile.name) {
            continue;
        }
        profile.id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        profile.created_at = now.clone();
        profile.updated_at = now;
        inner.profiles.push(profile);
        count += 1;
    }

    profile_service::save_profiles(&inner.profiles)?;
    Ok(count)
}

/// Get SSH key fingerprint (cached — ssh-keygen only runs once per key path)
#[tauri::command]
pub fn get_key_fingerprint(
    id: String,
    state: State<'_, AppState>,
) -> Result<KeyFingerprint, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id))?;

    let pub_path_str = profile.public_key_path.to_string_lossy().to_string();

    // Check cache first
    {
        let cache = FINGERPRINT_CACHE.lock().map_err(|_| MazeSshError::StateLockError)?;
        if let Some(cached) = cache.get(&pub_path_str) {
            return Ok(cached.clone());
        }
    }

    let fingerprint = compute_fingerprint(&profile.public_key_path)?;

    // Store in cache
    {
        let mut cache = FINGERPRINT_CACHE.lock().map_err(|_| MazeSshError::StateLockError)?;
        cache.insert(pub_path_str, fingerprint.clone());
    }

    Ok(fingerprint)
}

fn compute_fingerprint(pub_key_path: &PathBuf) -> Result<KeyFingerprint, MazeSshError> {
    let ssh_keygen = if cfg!(windows) {
        let system = std::path::Path::new("C:\\Windows\\System32\\OpenSSH\\ssh-keygen.exe");
        if system.exists() {
            system.to_string_lossy().to_string()
        } else {
            "ssh-keygen".to_string()
        }
    } else {
        "ssh-keygen".to_string()
    };

    let mut cmd = std::process::Command::new(&ssh_keygen);
    cmd.args(["-lf", &pub_key_path.to_string_lossy()]);

    // Hide console window on Windows
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .map_err(|e| MazeSshError::ConfigError(format!("ssh-keygen failed: {}", e)))?;

    if !output.status.success() {
        return Err(MazeSshError::ConfigError(
            "ssh-keygen could not read key".to_string(),
        ));
    }

    let line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Output format: "256 SHA256:abcdef... comment (ED25519)"
    let parts: Vec<&str> = line.splitn(4, ' ').collect();

    Ok(KeyFingerprint {
        bits: parts.first().unwrap_or(&"").to_string(),
        hash: parts.get(1).unwrap_or(&"").to_string(),
        comment: parts.get(2).unwrap_or(&"").to_string(),
        key_type: parts
            .get(3)
            .unwrap_or(&"")
            .trim_matches(|c| c == '(' || c == ')')
            .to_string(),
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyFingerprint {
    pub bits: String,
    pub hash: String,
    pub comment: String,
    pub key_type: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyHealthReport {
    pub profile_name: String,
    pub key_type: String,
    pub bits: u32,
    pub has_public_key: bool,
    pub has_passphrase: bool,
    pub issues: Vec<KeyHealthIssue>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyHealthIssue {
    pub severity: String,
    pub message: String,
}

/// Run a health check on all profile SSH keys
#[tauri::command]
pub fn check_all_keys_health(
    state: State<'_, AppState>,
) -> Result<Vec<KeyHealthReport>, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;

    let reports: Vec<KeyHealthReport> = inner
        .profiles
        .iter()
        .map(|profile| {
            let mut issues = Vec::new();

            if !profile.private_key_path.exists() {
                issues.push(KeyHealthIssue {
                    severity: "critical".to_string(),
                    message: "Private key file not found".to_string(),
                });
            }

            let has_public_key = profile.public_key_path.exists();
            if !has_public_key {
                issues.push(KeyHealthIssue {
                    severity: "warning".to_string(),
                    message: "Public key file not found".to_string(),
                });
            }

            if !profile.has_passphrase {
                issues.push(KeyHealthIssue {
                    severity: "warning".to_string(),
                    message: "Key has no passphrase protection".to_string(),
                });
            }

            let (key_type, bits) = if has_public_key {
                match compute_fingerprint(&profile.public_key_path) {
                    Ok(fp) => {
                        let bits_num = fp.bits.parse::<u32>().unwrap_or(0);
                        let key_type = fp.key_type.to_uppercase();

                        if key_type.contains("DSA") {
                            issues.push(KeyHealthIssue {
                                severity: "critical".to_string(),
                                message: "DSA keys are deprecated and insecure".to_string(),
                            });
                        } else if key_type.contains("RSA") && bits_num < 2048 {
                            issues.push(KeyHealthIssue {
                                severity: "critical".to_string(),
                                message: format!("RSA key too short ({} bits, minimum 2048)", bits_num),
                            });
                        } else if key_type.contains("RSA") && bits_num < 4096 {
                            issues.push(KeyHealthIssue {
                                severity: "info".to_string(),
                                message: format!("RSA {} bits — consider 4096 or Ed25519", bits_num),
                            });
                        }

                        (key_type, bits_num)
                    }
                    Err(_) => {
                        issues.push(KeyHealthIssue {
                            severity: "warning".to_string(),
                            message: "Could not read key fingerprint".to_string(),
                        });
                        ("Unknown".to_string(), 0)
                    }
                }
            } else {
                ("Unknown".to_string(), 0)
            };

            KeyHealthReport {
                profile_name: profile.name.clone(),
                key_type,
                bits,
                has_public_key,
                has_passphrase: profile.has_passphrase,
                issues,
            }
        })
        .collect();

    Ok(reports)
}

/// Read the public key content for clipboard copy
#[tauri::command]
pub fn read_public_key(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id))?;

    if !profile.public_key_path.exists() {
        return Err(MazeSshError::KeyNotFound(profile.public_key_path.clone()));
    }

    let content = std::fs::read_to_string(&profile.public_key_path)
        .map_err(|e| MazeSshError::IoError(e))?;
    Ok(content.trim().to_string())
}
