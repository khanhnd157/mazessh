use std::path::PathBuf;
use tauri::{Emitter, Manager, State};

use crate::commands::security::ensure_unlocked;
use crate::commands::switch::ActivationResult;
use crate::error::MazeSshError;
use crate::models::repo_mapping::{GitConfigScope, RepoMappingSummary};
use crate::services::{
    git_identity_service, profile_service, repo_detection_service, security as security_service, ssh_engine,
};
use crate::state::AppState;

#[tauri::command]
pub fn resolve_repo_path(path: String, state: State<'_, AppState>) -> Result<Option<String>, MazeSshError> {
    ensure_unlocked(&state)?;
    let p = PathBuf::from(&path);
    Ok(repo_detection_service::find_git_root(&p)
        .map(|r| r.to_string_lossy().to_string()))
}

#[tauri::command]
pub fn check_repo_mapping(
    path: String,
    state: State<'_, AppState>,
) -> Result<Option<RepoMappingSummary>, MazeSshError> {
    ensure_unlocked(&state)?;
    let p = PathBuf::from(&path);
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;

    let git_root = match repo_detection_service::find_git_root(&p) {
        Some(r) => r,
        None => return Ok(None),
    };

    let mapping = match repo_detection_service::lookup_mapping(&git_root, &inner.repo_mappings) {
        Some(m) => m,
        None => return Ok(None),
    };

    let profile_name = inner
        .profiles
        .iter()
        .find(|p| p.id == mapping.profile_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(Some(RepoMappingSummary {
        id: mapping.id.clone(),
        repo_path: mapping.repo_path.to_string_lossy().to_string(),
        repo_name: mapping.repo_name.clone(),
        profile_id: mapping.profile_id.clone(),
        profile_name,
        git_config_scope: mapping.git_config_scope.clone(),
    }))
}

#[tauri::command]
pub async fn auto_switch_for_repo(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<ActivationResult>, MazeSshError> {
    ensure_unlocked(&state)?;
    let p = PathBuf::from(&path);

    // Increment activation counter to detect race with concurrent activations
    let our_seq = {
        let mut security = state.security.lock().map_err(|_| MazeSshError::StateLockError)?;
        security.activation_counter = security.activation_counter.wrapping_add(1);
        security.activation_counter
    };

    // Extract everything we need from state
    let (profile, mapping_scope, git_root, key_path, profile_id) = {
        let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;

        let git_root = match repo_detection_service::find_git_root(&p) {
            Some(r) => r,
            None => return Ok(None),
        };

        let mapping =
            match repo_detection_service::lookup_mapping(&git_root, &inner.repo_mappings) {
                Some(m) => m.clone(),
                None => return Ok(None),
            };

        let profile = match inner.profiles.iter().find(|p| p.id == mapping.profile_id) {
            Some(p) => p.clone(),
            None => return Ok(None),
        };

        let key_path = profile.private_key_path.to_string_lossy().to_string();
        let profile_id = profile.id.clone();
        (profile, mapping.git_config_scope, git_root, key_path, profile_id)
    };

    // Activate the profile (same as manual activation)
    {
        let mut inner = state.inner.write().map_err(|_| MazeSshError::StateLockError)?;
        inner.active_profile_id = Some(profile.id.clone());
    }

    profile_service::save_active_profile_id(Some(&profile.id))?;
    ssh_engine::write_env_file(&profile).map_err(MazeSshError::IoError)?;

    let git_ssh_command = ssh_engine::build_git_ssh_command(&profile);
    let profile_name = profile.name.clone();

    // Background: agent + env var + git identity sync
    let profile_bg = profile.clone();
    let git_root_bg = git_root.clone();
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

        // Set env var
        match ssh_engine::set_user_env_git_ssh_command(&profile_bg) {
            Ok(()) => status_parts.push("GIT_SSH_COMMAND set".to_string()),
            Err(e) => status_parts.push(format!("Env var failed: {}", e)),
        }

        // SSH agent
        let passphrase = if profile_bg.has_passphrase {
            security_service::get_passphrase(&profile_id).ok().flatten()
        } else {
            None
        };
        match ssh_engine::ensure_agent_running() {
            Ok(true) => match ssh_engine::agent_switch_key(&key_path, passphrase.as_deref()) {
                Ok(_) => status_parts.push("Key loaded into ssh-agent".to_string()),
                Err(e) => status_parts.push(format!("ssh-add failed: {}", e)),
            },
            Ok(false) => status_parts.push("Could not start ssh-agent".to_string()),
            Err(e) => status_parts.push(format!("Agent error: {}", e)),
        }

        // Git identity sync
        match &mapping_scope {
            GitConfigScope::Local => {
                match git_identity_service::set_git_identity_local(
                    &git_root_bg,
                    &profile_bg.git_username,
                    &profile_bg.email,
                ) {
                    Ok(()) => status_parts.push(format!(
                        "Git identity set (local: {})",
                        profile_bg.git_username
                    )),
                    Err(e) => status_parts.push(format!("Git identity failed: {}", e)),
                }
            }
            GitConfigScope::Global => {
                match git_identity_service::set_git_identity_global(
                    &profile_bg.git_username,
                    &profile_bg.email,
                ) {
                    Ok(()) => status_parts.push(format!(
                        "Git identity set (global: {})",
                        profile_bg.git_username
                    )),
                    Err(e) => status_parts.push(format!("Git identity failed: {}", e)),
                }
            }
        }

        let success = status_parts.iter().any(|s| s.contains("loaded"));
        let status = status_parts.join(" | ");

        let _ = app_bg.emit(
            "agent-status",
            crate::commands::switch::AgentStatusEvent {
                status,
                success,
            },
        );
    });

    Ok(Some(ActivationResult {
        profile_name,
        git_ssh_command,
    }))
}
