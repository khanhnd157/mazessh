/// Relay binary download and version tracking service.
///
/// Downloads npiperelay.exe and wsl-ssh-pageant.exe from GitHub Releases,
/// tracks installed versions in ~/.maze-ssh/bin/bin-version.json.
use std::path::PathBuf;

use serde_json::Value;

use crate::error::MazeSshError;
use crate::models::bridge_provider::{BinaryVersion, DownloadProgress, RelayBinary};
use crate::services::profile_service;

#[cfg(feature = "desktop")]
use tauri::{AppHandle, Emitter};

fn bin_dir() -> PathBuf {
    profile_service::data_dir().join("bin")
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

/// Fetch the latest release metadata from GitHub API.
/// Returns `(tag_name, download_url)` for the target asset.
async fn fetch_release_info(binary: RelayBinary) -> Result<(String, String), MazeSshError> {
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

    Ok((tag, download_url))
}

// ── Download with progress ──

#[cfg(feature = "desktop")]
pub async fn download_binary(binary: RelayBinary, app: &AppHandle) -> Result<(), MazeSshError> {
    use futures_util::StreamExt;

    let binary_name = binary.version_key().to_string();

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

    // Step 1: fetch release info
    let (tag, download_url) = fetch_release_info(binary).await.map_err(|e| {
        emit_progress(0, "error");
        e
    })?;

    // Step 2: stream download
    let client = reqwest::Client::builder()
        .user_agent("maze-ssh/1.0")
        .build()
        .map_err(|e| MazeSshError::BridgeError(format!("HTTP client error: {e}")))?;

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

    // Step 3: write to disk
    let dest = bin_dir().join(binary.filename());
    std::fs::create_dir_all(bin_dir())?;
    std::fs::write(&dest, &bytes).map_err(|e| {
        emit_progress(0, "error");
        MazeSshError::BridgeError(format!(
            "Failed to write {}: {e}",
            dest.display()
        ))
    })?;

    // Step 4: record version
    record_version(binary, &tag)?;

    emit_progress(100, "done");
    Ok(())
}

/// Non-desktop stub so the lib compiles for CLI
#[cfg(not(feature = "desktop"))]
pub async fn download_binary(_binary: RelayBinary, _app: &()) -> Result<(), MazeSshError> {
    Err(MazeSshError::BridgeError("Not available in CLI mode".to_string()))
}
