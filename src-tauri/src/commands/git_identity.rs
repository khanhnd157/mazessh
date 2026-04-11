use std::path::PathBuf;
use tauri::State;

use crate::error::MazeSshError;
use crate::models::repo_mapping::{GitConfigScope, GitIdentityInfo};
use crate::services::git_identity_service;
use crate::state::AppState;

#[tauri::command]
pub fn get_current_git_identity() -> Result<GitIdentityInfo, MazeSshError> {
    git_identity_service::get_git_identity_global()
}

#[tauri::command]
pub fn get_repo_git_identity(repo_path: String) -> Result<GitIdentityInfo, MazeSshError> {
    git_identity_service::get_git_identity_local(&PathBuf::from(repo_path))
}

#[tauri::command]
pub fn sync_git_identity(
    profile_id: String,
    repo_path: Option<String>,
    scope: GitConfigScope,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    let inner = state.inner.lock().unwrap();
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(profile_id))?;

    let name = profile.git_username.clone();
    let email = profile.email.clone();
    drop(inner);

    match scope {
        GitConfigScope::Global => {
            git_identity_service::set_git_identity_global(&name, &email)?;
        }
        GitConfigScope::Local => {
            let path = repo_path.ok_or_else(|| {
                MazeSshError::GitConfigError("repo_path required for local scope".to_string())
            })?;
            git_identity_service::set_git_identity_local(&PathBuf::from(path), &name, &email)?;
        }
    }

    Ok(())
}
