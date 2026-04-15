/// Relay binary download and version tracking service.
///
/// Downloads npiperelay.exe and wsl-ssh-pageant.exe from GitHub Releases,
/// tracks installed versions in ~/.maze-ssh/bin/bin-version.json.
///
/// # Integrity verification
/// After downloading a binary the SHA256 digest is computed and compared against
/// a companion `<asset>.sha256` file published in the same GitHub Release.
/// If the upstream repo does not publish a checksum file the download still
/// proceeds but a `binary-download-warning` event is emitted so the UI can
/// alert the user. A hash mismatch is always a hard error and the file is
/// never written to disk.
use std::path::PathBuf;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::MazeSshError;
use crate::models::bridge_provider::{BinaryUpdateStatus, BinaryVersion, DownloadProgress, RelayBinary};
use crate::services::profile_service;

#[cfg(feature = "desktop")]
use tauri::{AppHandle, Emitter};

fn bin_dir() -> PathBuf {
    profile_service::data_dir()
        .unwrap_or_else(|_| PathBuf::from(".maze-ssh"))
        .join("bin")
}

fn version_file() -> PathBuf {
    bin_dir().join("bin-version.json")
}

/// Read the installed binary versions from disk.
/// Returns default (all None) if the file doesn't exist or can't be parsed.
pub fn get_installed_versions() -> BinaryVersion {
    match std::fs::read_to_string(version_file()) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => BinaryVersion::default(),
    }
}

fn save_versions(versions: &BinaryVersion) -> Result<(), MazeSshError> {
    let dir = bin_dir();
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(versions)?;
    profile_service::atomic_write(&version_file(), &content)?;
    Ok(())
}

/// Set the installed version for one binary and persist.
fn record_version(binary: RelayBinary, tag: &str) -> Result<(), MazeSshError> {
    let mut versions = get_installed_versions();
    match binary {
        RelayBinary::Npiperelay => versions.npiperelay = Some(tag.to_string()),
        RelayBinary::WslSshPageant => versions.wsl_ssh_pageant = Some(tag.to_string()),
    }
    save_versions(&versions)
}

// ── GitHub API helpers ──

/// Metadata returned from a GitHub Release lookup.
struct ReleaseInfo {
    tag: String,
    download_url: String,
    /// URL of a companion `<asset>.sha256` file, if the repo publishes one.
    checksum_url: Option<String>,
}

/// Fetch the latest release metadata from GitHub API.
/// Returns `ReleaseInfo` for the target asset, including the checksum URL when available.
async fn fetch_release_info(binary: RelayBinary) -> Result<ReleaseInfo, MazeSshError> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        binary.github_repo()
    );

    let client = reqwest::Client::builder()
        .user_agent("maze-ssh/1.0")
        .build()
        .map_err(|e| MazeSshError::BridgeError(format!("Failed to build HTTP client: {e}")))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| MazeSshError::BridgeError(format!("GitHub API request failed: {e}")))?;

    if !response.status().is_success() {
        return Err(MazeSshError::BridgeError(format!(
            "GitHub API returned {}: {}",
            response.status(),
            url
        )));
    }

    let json: Value = response
        .json()
        .await
        .map_err(|e| MazeSshError::BridgeError(format!("Failed to parse GitHub response: {e}")))?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| MazeSshError::BridgeError("No tag_name in GitHub response".to_string()))?
        .to_string();

    let asset_name = binary.asset_name();
    let assets = json["assets"]
        .as_array()
        .ok_or_else(|| MazeSshError::BridgeError("No assets in GitHub response".to_string()))?;

    let download_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(asset_name))
        .and_then(|a| a["browser_download_url"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            MazeSshError::BridgeError(format!(
                "Asset '{}' not found in release {}",
                asset_name, tag
            ))
        })?;

    // Look for a companion checksum file: "<asset>.sha256" or "sha256sums.txt"
    let checksum_asset_name = format!("{}.sha256", asset_name);
    let checksum_url = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name == checksum_asset_name || name == "sha256sums.txt" || name == "checksums.txt"
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .map(|s| s.to_string());

    Ok(ReleaseInfo { tag, download_url, checksum_url })
}

/// Download a text file and return its content (used for checksum files).
async fn download_text(client: &reqwest::Client, url: &str) -> Result<String, MazeSshError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| MazeSshError::BridgeError(format!("Checksum download failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(MazeSshError::BridgeError(format!(
            "Checksum URL returned HTTP {}",
            resp.status()
        )));
    }

    resp.text()
        .await
        .map_err(|e| MazeSshError::BridgeError(format!("Failed to read checksum response: {e}")))
}

/// Compute SHA256 of `data` and return it as a lowercase hex string.
fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// Extract the expected SHA256 hex for `asset_name` from a checksum file.
///
/// Supports two common formats:
/// - Single-file `<asset>.sha256`: the entire file is just the hex digest (optionally followed
///   by whitespace and a filename).
/// - Multi-file `sha256sums.txt` / `checksums.txt`: each line is
///   `<hex>  <filename>` or `<hex> *<filename>` (GNU coreutils / BSD shasum style).
fn parse_expected_checksum(checksum_text: &str, asset_name: &str) -> Option<String> {
    let text = checksum_text.trim();

    // Single-value file: whole file is just the hex hash (64 hex chars)
    if text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(text.to_ascii_lowercase());
    }

    // Multi-line: find the line matching asset_name
    for line in text.lines() {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() == 2 {
            let hash = parts[0].trim();
            // filename may be prefixed with '*' (binary mode marker)
            let fname = parts[1].trim().trim_start_matches('*');
            if fname == asset_name && hash.len() == 64 {
                return Some(hash.to_ascii_lowercase());
            }
        }
    }
    None
}

// ── Download with progress ──

#[cfg(feature = "desktop")]
pub async fn download_binary(binary: RelayBinary, app: &AppHandle) -> Result<(), MazeSshError> {
    use futures_util::StreamExt;

    let binary_name = binary.version_key().to_string();
    let asset_name = binary.asset_name();

    let emit_progress = |percent: u8, status: &str| {
        let _ = app.emit(
            "binary-download-progress",
            DownloadProgress {
                binary: binary_name.clone(),
                percent,
                status: status.to_string(),
            },
        );
    };

    emit_progress(0, "downloading");

    // Step 1: fetch release info (includes checksum URL when available)
    let info = fetch_release_info(binary).await.map_err(|e| {
        emit_progress(0, "error");
        e
    })?;
    let ReleaseInfo { tag, download_url, checksum_url } = info;

    let client = reqwest::Client::builder()
        .user_agent("maze-ssh/1.0")
        .build()
        .map_err(|e| MazeSshError::BridgeError(format!("HTTP client error: {e}")))?;

    // Step 2: download expected checksum (best-effort)
    let expected_checksum: Option<String> = match checksum_url {
        Some(ref url) => {
            match download_text(&client, url).await {
                Ok(text) => {
                    let parsed = parse_expected_checksum(&text, asset_name);
                    if parsed.is_none() {
                        eprintln!(
                            "[relay-bundler] checksum file found but could not parse hash \
                             for asset '{}' — will warn after download",
                            asset_name
                        );
                    }
                    parsed
                }
                Err(e) => {
                    eprintln!("[relay-bundler] failed to fetch checksum file: {e}");
                    None
                }
            }
        }
        None => None,
    };

    // Warn early if no checksum is available (non-blocking — upstream may not publish one)
    if expected_checksum.is_none() {
        eprintln!(
            "[relay-bundler] WARNING: no checksum available for {} {} — \
             integrity cannot be verified",
            asset_name, tag
        );
        let _ = app.emit(
            "binary-download-warning",
            serde_json::json!({
                "binary": binary_name,
                "tag": tag,
                "message": format!(
                    "No SHA256 checksum found for {} {}. \
                     The binary will be downloaded but its integrity cannot be verified. \
                     Only proceed if you trust the source.",
                    asset_name, tag
                ),
            }),
        );
    }

    // Step 3: stream download
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| {
            emit_progress(0, "error");
            MazeSshError::BridgeError(format!("Download request failed: {e}"))
        })?;

    if !response.status().is_success() {
        emit_progress(0, "error");
        return Err(MazeSshError::BridgeError(format!(
            "Download failed with HTTP {}",
            response.status()
        )));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_percent: u8 = 0;
    let mut bytes: Vec<u8> = Vec::with_capacity(total_size as usize + 1);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            emit_progress(0, "error");
            MazeSshError::BridgeError(format!("Download stream error: {e}"))
        })?;
        bytes.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = ((downloaded * 100) / total_size).min(99) as u8;
            if percent >= last_percent + 5 {
                last_percent = percent;
                emit_progress(percent, "downloading");
            }
        }
    }

    // Step 4: verify SHA256 integrity before touching disk
    let actual_hash = sha256_hex(&bytes);
    if let Some(ref expected) = expected_checksum {
        if actual_hash != *expected {
            emit_progress(0, "error");
            return Err(MazeSshError::BridgeError(format!(
                "Integrity check FAILED for {} {}: \
                 expected SHA256 {}, got {}. \
                 The file has NOT been saved. This may indicate a supply-chain attack.",
                asset_name, tag, expected, actual_hash
            )));
        }
        eprintln!(
            "[relay-bundler] SHA256 verified OK for {} {}: {}",
            asset_name, tag, actual_hash
        );
    }

    // Step 5: write to disk only after successful (or unavailable) integrity check
    let dest = bin_dir().join(binary.filename());
    std::fs::create_dir_all(bin_dir())?;
    std::fs::write(&dest, &bytes).map_err(|e| {
        emit_progress(0, "error");
        MazeSshError::BridgeError(format!(
            "Failed to write {}: {e}",
            dest.display()
        ))
    })?;

    // Step 6: record version and computed hash for future reference
    record_version(binary, &tag)?;

    emit_progress(100, "done");
    Ok(())
}

/// Non-desktop stub so the lib compiles for CLI
#[cfg(not(feature = "desktop"))]
pub async fn download_binary(_binary: RelayBinary, _app: &()) -> Result<(), MazeSshError> {
    Err(MazeSshError::BridgeError("Not available in CLI mode".to_string()))
}

// ── Update check ──

/// Check for available updates for all relay binaries.
/// Compares installed version (from bin-version.json) against GitHub latest release.
/// On network error, returns entries with latest_version=None, update_available=false.
pub async fn check_for_updates() -> Vec<BinaryUpdateStatus> {
    let installed = get_installed_versions();
    let mut results = Vec::new();

    for binary in RelayBinary::all() {
        let installed_version = match binary {
            RelayBinary::Npiperelay => installed.npiperelay.clone(),
            RelayBinary::WslSshPageant => installed.wsl_ssh_pageant.clone(),
        };

        let latest_version = fetch_release_info(*binary)
            .await
            .map(|info| info.tag)
            .ok();

        let update_available = match (&installed_version, &latest_version) {
            (Some(inst), Some(latest)) => inst != latest,
            _ => false,
        };

        results.push(BinaryUpdateStatus {
            binary: binary.version_key().to_string(),
            installed_version,
            latest_version,
            update_available,
        });
    }

    results
}
