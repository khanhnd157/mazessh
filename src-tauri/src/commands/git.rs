use serde::Serialize;
use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::services::ssh_engine;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub output: String,
    pub profile_name: String,
}

#[tauri::command]
pub fn get_git_ssh_command(
    id: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.lock().unwrap();
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id))?;
    Ok(ssh_engine::build_git_ssh_command(profile))
}

#[tauri::command]
pub async fn test_ssh_connection(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionTestResult, MazeSshError> {
    ensure_unlocked(&state)?;
    let (profile_name, hostname, key_path, port) = {
        let inner = state.inner.lock().unwrap();
        let profile = inner
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| MazeSshError::ProfileNotFound(id))?
            .clone();
        let port = profile.port_or_default();
        let key = profile.private_key_path.to_string_lossy().to_string();
        (profile.name, profile.hostname, key, port)
    };

    // Build ssh args with the specific identity key
    let mut args = vec![
        "-T".to_string(),
        "-i".to_string(),
        key_path.clone(),
        "-o".to_string(),
        "IdentitiesOnly=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
    ];
    if port != 22 {
        args.push("-p".to_string());
        args.push(port.to_string());
    }
    args.push(format!("git@{}", hostname));

    // On Windows, prefer the system OpenSSH
    #[cfg(windows)]
    let ssh_bin = {
        let system_ssh = std::path::Path::new("C:\\Windows\\System32\\OpenSSH\\ssh.exe");
        if system_ssh.exists() {
            system_ssh.to_string_lossy().to_string()
        } else {
            "ssh".to_string()
        }
    };
    #[cfg(not(windows))]
    let ssh_bin = "ssh".to_string();

    let mut cmd = tokio::process::Command::new(&ssh_bin);
    cmd.args(&args);

    // Hide console window on Windows
    #[cfg(windows)]
    {
        #[allow(unused_imports)]
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| MazeSshError::ConnectionFailed(format!("Failed to run {}: {}", ssh_bin, e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    let combined_output = format!("{}{}", stdout, stderr);

    // GitHub/GitLab/Gitea return exit code 1 with a success message
    let success = combined_output.contains("successfully authenticated")
        || combined_output.contains("Welcome to GitLab")
        || combined_output.contains("Welcome to Gitea")
        || combined_output.contains("Hi ")
        || output.status.success();

    let display_output = if combined_output.trim().is_empty() {
        format!("SSH command: {} {}\n(no output, exit code: {})", ssh_bin, args.join(" "), exit_code)
    } else {
        combined_output.trim().to_string()
    };

    Ok(ConnectionTestResult {
        success,
        output: display_output,
        profile_name,
    })
}
