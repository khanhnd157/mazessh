use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::models::profile::ProfileSummary;
use crate::models::security::AgentMode;
use crate::services::{git_identity_service, profile_service, security as security_service, ssh_engine};
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
    ensure_unlocked(&state)?;

    // Mark agent activation time for session timeout, and capture sequence number
    let our_seq = {
        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.agent_activated_at = Some(std::time::Instant::now());
        security.activation_counter = security.activation_counter.wrapping_add(1);
        security.activation_counter
    };

    // Determine agent mode
    let agent_mode = state
        .security
        .lock()
        .map(|s| s.settings.agent_mode)
        .unwrap_or(AgentMode::FileSystem);

    // Step 1: Quick state update (instant)
    let (profile, git_ssh_command, key_path) = {
        let mut inner = state.inner.write().map_err(|_| MazeSshError::StateLockError)?;
        let profile = inner
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| MazeSshError::ProfileNotFound(id.clone()))?
            .clone();

        inner.active_profile_id = Some(id.clone());

        // Use agent pipe when vault mode, file-based when filesystem mode
        let git_ssh_command = match agent_mode {
            AgentMode::Vault => ssh_engine::build_git_ssh_command_agent(&profile),
            AgentMode::FileSystem => ssh_engine::build_git_ssh_command(&profile),
        };
        let key_path = profile.private_key_path.to_string_lossy().to_string();
        (profile, git_ssh_command, key_path)
    };

    // Step 2: Save to disk (fast)
    profile_service::save_active_profile_id(Some(&id))?;
    // Write agent-aware env file
    match agent_mode {
        AgentMode::Vault => {
            let content = ssh_engine::build_env_file_content_agent(&profile);
            let home = dirs::home_dir().ok_or_else(|| MazeSshError::ConfigError("Home not found".into()))?;
            std::fs::write(home.join(".maze-ssh").join("env"), content).map_err(MazeSshError::IoError)?;
        }
        AgentMode::FileSystem => {
            ssh_engine::write_env_file(&profile).map_err(MazeSshError::IoError)?;
        }
    }

    let profile_name = profile.name.clone();
    let profile_for_bg = profile.clone();
    let profile_id_for_bg = id.clone();

    // Step 3: Heavy agent/env work in background (doesn't block UI)
    let app_bg = app.clone();
    tokio::task::spawn_blocking(move || {
        // Bail out if a newer activation has superseded this one
        let still_latest = app_bg.state::<AppState>()
            .security.lock()
            .map(|s| s.activation_counter == our_seq)
            .unwrap_or(false);
        if !still_latest {
            return;
        }

        let mut status_parts: Vec<String> = Vec::new();

        // Set persistent user environment variable (uses the agent-aware command)
        let env_cmd = match agent_mode {
            AgentMode::Vault => ssh_engine::build_git_ssh_command_agent(&profile_for_bg),
            AgentMode::FileSystem => ssh_engine::build_git_ssh_command(&profile_for_bg),
        };
        match ssh_engine::set_user_env_git_ssh_command_value(&env_cmd) {
            Ok(()) => status_parts.push("GIT_SSH_COMMAND set".to_string()),
            Err(e) => status_parts.push(format!("Env var failed: {}", e)),
        }

        if agent_mode == AgentMode::Vault {
            // Vault mode: MazeSSH Agent handles signing via named pipe
            status_parts.push("Using MazeSSH Agent (vault mode)".to_string());
        } else {
            // FileSystem mode: load key into Windows ssh-agent via ssh-add
            let passphrase = if profile_for_bg.has_passphrase {
                security_service::get_passphrase(&profile_id_for_bg).ok().flatten()
            } else {
                None
            };
            match ssh_engine::ensure_agent_running() {
                Ok(true) => match ssh_engine::agent_switch_key(&key_path, passphrase.as_deref()) {
                    Ok(_) => status_parts.push("Key loaded into ssh-agent".to_string()),
                    Err(e) => status_parts.push(format!("ssh-add failed: {}", e)),
                },
                Ok(false) => {
                    status_parts.push("Could not start ssh-agent (may need admin)".to_string())
                }
                Err(e) => status_parts.push(format!("Agent error: {}", e)),
            }
        }

        // Sync global git identity
        match git_identity_service::set_git_identity_global(
            &profile_for_bg.git_username,
            &profile_for_bg.email,
        ) {
            Ok(()) => status_parts.push(format!("Git: {}", profile_for_bg.git_username)),
            Err(e) => status_parts.push(format!("Git identity failed: {}", e)),
        }

        let success = status_parts.iter().any(|s| s.contains("MazeSSH Agent") || s.contains("loaded"));
        let status = status_parts.join(" | ");

        let _ = app_bg.emit(
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
    ensure_unlocked(&state)?;

    // Clear agent activation time
    {
        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.agent_activated_at = None;
    }

    // Quick state update
    {
        let mut inner = state.inner.write().map_err(|_| MazeSshError::StateLockError)?;
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
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    if let Some(active_id) = &inner.active_profile_id {
        let profile = inner.profiles.iter().find(|p| p.id == *active_id);
        Ok(profile.map(|p| ProfileSummary::from_profile(p, &inner.active_profile_id)))
    } else {
        Ok(None)
    }
}
