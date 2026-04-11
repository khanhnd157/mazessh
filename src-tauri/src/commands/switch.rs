use serde::Serialize;
use tauri::State;

use crate::error::MazeSshError;
use crate::models::profile::ProfileSummary;
use crate::services::{profile_service, ssh_engine};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ActivationResult {
    pub profile_name: String,
    pub git_ssh_command: String,
}

#[tauri::command]
pub fn activate_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<ActivationResult, MazeSshError> {
    let mut inner = state.inner.lock().unwrap();
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id.clone()))?
        .clone();

    inner.active_profile_id = Some(id.clone());
    profile_service::save_active_profile_id(Some(&id))?;

    let git_ssh_command = ssh_engine::build_git_ssh_command(&profile);
    ssh_engine::write_env_file(&profile)
        .map_err(|e| MazeSshError::IoError(e))?;

    Ok(ActivationResult {
        profile_name: profile.name,
        git_ssh_command,
    })
}

#[tauri::command]
pub fn deactivate_profile(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let mut inner = state.inner.lock().unwrap();
    inner.active_profile_id = None;
    profile_service::save_active_profile_id(None)?;
    ssh_engine::clear_env_file()
        .map_err(|e| MazeSshError::IoError(e))?;
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
