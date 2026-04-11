use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::error::MazeSshError;
use crate::models::profile::ProfileSummary;
use crate::services::{profile_service, ssh_engine};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ActivationResult {
    pub profile_name: String,
    pub git_ssh_command: String,
}

/// Event payload emitted when background agent work completes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusEvent {
    pub status: String,
    pub success: bool,
}

#[tauri::command]
pub async fn activate_profile(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ActivationResult, MazeSshError> {
    // Step 1: Quick state update (instant)
    let (profile, git_ssh_command, key_path) = {
        let mut inner = state.inner.lock().unwrap();
        let profile = inner
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| MazeSshError::ProfileNotFound(id.clone()))?
            .clone();

        inner.active_profile_id = Some(id.clone());

        let git_ssh_command = ssh_engine::build_git_ssh_command(&profile);
        let key_path = profile.private_key_path.to_string_lossy().to_string();
        (profile, git_ssh_command, key_path)
    };

    // Step 2: Save to disk (fast)
    profile_service::save_active_profile_id(Some(&id))?;
    ssh_engine::write_env_file(&profile).map_err(MazeSshError::IoError)?;

    let profile_name = profile.name.clone();
    let profile_for_bg = profile.clone();

    // Step 3: Heavy agent/env work in background (doesn't block UI)
    tokio::task::spawn_blocking(move || {
        let mut status_parts: Vec<String> = Vec::new();

        // Set persistent user environment variable
        match ssh_engine::set_user_env_git_ssh_command(&profile_for_bg) {
            Ok(()) => status_parts.push("GIT_SSH_COMMAND set".to_string()),
            Err(e) => status_parts.push(format!("Env var failed: {}", e)),
        }

        // Start SSH agent and add key
        match ssh_engine::ensure_agent_running() {
            Ok(true) => match ssh_engine::agent_switch_key(&key_path) {
                Ok(_) => status_parts.push("Key loaded into ssh-agent".to_string()),
                Err(e) => status_parts.push(format!("ssh-add failed: {}", e)),
            },
            Ok(false) => {
                status_parts.push("Could not start ssh-agent (may need admin)".to_string())
            }
            Err(e) => status_parts.push(format!("Agent error: {}", e)),
        }

        let success = status_parts.iter().any(|s| s.contains("loaded"));
        let status = status_parts.join(" | ");

        let _ = app.emit(
            "agent-status",
            AgentStatusEvent {
                status: status.clone(),
                success,
            },
        );
    });

    Ok(ActivationResult {
        profile_name,
        git_ssh_command,
    })
}

#[tauri::command]
pub async fn deactivate_profile(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), MazeSshError> {
    // Quick state update
    {
        let mut inner = state.inner.lock().unwrap();
        inner.active_profile_id = None;
    }

    profile_service::save_active_profile_id(None)?;
    ssh_engine::clear_env_file().map_err(MazeSshError::IoError)?;

    // Background cleanup
    tokio::task::spawn_blocking(move || {
        let _ = ssh_engine::clear_user_env_git_ssh_command();
        let _ = ssh_engine::agent_clear_keys();

        let _ = app.emit(
            "agent-status",
            AgentStatusEvent {
                status: "Deactivated — agent keys cleared".to_string(),
                success: true,
            },
        );
    });

    Ok(())
}

#[tauri::command]
pub fn get_active_profile(
    state: State<'_, AppState>,
) -> Result<Option<ProfileSummary>, MazeSshError> {
    let inner = state.inner.lock().unwrap();
    if let Some(active_id) = &inner.active_profile_id {
        let profile = inner.profiles.iter().find(|p| p.id == *active_id);
        Ok(profile.map(|p| ProfileSummary::from_profile(p, &inner.active_profile_id)))
    } else {
        Ok(None)
    }
}
