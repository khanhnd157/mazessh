use serde::Serialize;
use tauri::State;

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
    let (profile_name, hostname, ssh_command) = {
        let inner = state.inner.lock().unwrap();
        let profile = inner
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| MazeSshError::ProfileNotFound(id))?
            .clone();
        let ssh_cmd = ssh_engine::build_git_ssh_command(&profile);
        (profile.name, profile.hostname, ssh_cmd)
    };

    let output = tokio::process::Command::new("ssh")
        .args([
            "-T",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=10",
            &format!("git@{}", hostname),
        ])
        .env("GIT_SSH_COMMAND", &ssh_command)
        .output()
        .await
        .map_err(|e| MazeSshError::ConnectionFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    // GitHub/GitLab return exit code 1 with a success message
    let success = combined.contains("successfully authenticated")
        || combined.contains("Welcome to GitLab")
        || combined.contains("Hi ")
        || output.status.success();

    Ok(ConnectionTestResult {
        success,
        output: combined.trim().to_string(),
        profile_name,
    })
}
