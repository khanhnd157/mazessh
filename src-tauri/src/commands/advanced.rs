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
