use std::path::PathBuf;
use tauri::State;

use crate::error::MazeSshError;
use crate::models::profile::{
    CreateProfileInput, ProfileSummary, SshProfile, UpdateProfileInput,
};
use crate::commands::security::ensure_unlocked;
use crate::services::{profile_service, repo_mapping_service, security};
use crate::state::AppState;

#[tauri::command]
pub fn get_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileSummary>, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.lock().unwrap();
    let summaries = inner
        .profiles
        .iter()
        .map(|p| ProfileSummary::from_profile(p, &inner.active_profile_id))
        .collect();
    Ok(summaries)
}

#[tauri::command]
pub fn get_profile(id: String, state: State<'_, AppState>) -> Result<SshProfile, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.lock().unwrap();
    inner
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| MazeSshError::ProfileNotFound(id))
}

#[tauri::command]
pub fn create_profile(
    input: CreateProfileInput,
    state: State<'_, AppState>,
) -> Result<SshProfile, MazeSshError> {
    ensure_unlocked(&state)?;
    let private_key_path = PathBuf::from(&input.private_key_path);
    if !private_key_path.exists() {
        return Err(MazeSshError::KeyNotFound(private_key_path));
    }

    let public_key_path = PathBuf::from(format!("{}.pub", input.private_key_path));

    let now = chrono::Utc::now().to_rfc3339();
    let profile = SshProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        provider: input.provider,
        email: input.email,
        git_username: input.git_username,
        private_key_path,
        public_key_path,
        host_alias: input.host_alias,
        hostname: input.hostname,
        port: input.port,
        ssh_user: input.ssh_user,
        has_passphrase: input.has_passphrase,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut inner = state.inner.lock().unwrap();
    inner.profiles.push(profile.clone());
    profile_service::save_profiles(&inner.profiles)?;

    Ok(profile)
}

#[tauri::command]
pub fn update_profile(
    id: String,
    input: UpdateProfileInput,
    state: State<'_, AppState>,
) -> Result<SshProfile, MazeSshError> {
    ensure_unlocked(&state)?;
    let mut inner = state.inner.lock().unwrap();
    let profile = inner
        .profiles
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id))?;

    if let Some(name) = input.name {
        profile.name = name;
    }
    if let Some(provider) = input.provider {
        profile.provider = provider;
    }
    if let Some(email) = input.email {
        profile.email = email;
    }
    if let Some(git_username) = input.git_username {
        profile.git_username = git_username;
    }
    if let Some(host_alias) = input.host_alias {
        profile.host_alias = host_alias;
    }
    if let Some(hostname) = input.hostname {
        profile.hostname = hostname;
    }
    if let Some(port) = input.port {
        profile.port = Some(port);
    }
    if let Some(ssh_user) = input.ssh_user {
        profile.ssh_user = Some(ssh_user);
    }
    profile.updated_at = chrono::Utc::now().to_rfc3339();

    let updated = profile.clone();
    profile_service::save_profiles(&inner.profiles)?;

    Ok(updated)
}

#[tauri::command]
pub fn delete_profile(id: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut inner = state.inner.lock().unwrap();
    let idx = inner
        .profiles
        .iter()
        .position(|p| p.id == id)
        .ok_or_else(|| MazeSshError::ProfileNotFound(id.clone()))?;

    inner.profiles.remove(idx);

    // Clear active if deleted
    if inner.active_profile_id.as_ref() == Some(&id) {
        inner.active_profile_id = None;
        profile_service::save_active_profile_id(None)?;
    }

    profile_service::save_profiles(&inner.profiles)?;

    // Cascade: remove repo mappings for this profile
    let had_mappings = inner.repo_mappings.iter().any(|m| m.profile_id == id);
    inner.repo_mappings.retain(|m| m.profile_id != id);
    if had_mappings {
        let _ = repo_mapping_service::save_mappings(&inner.repo_mappings);
    }

    // Remove passphrase from keyring
    let _ = security::delete_passphrase(&id);

    Ok(())
}
