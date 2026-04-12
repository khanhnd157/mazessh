use std::path::PathBuf;
use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::models::repo_mapping::{
    CreateRepoMappingInput, GitConfigScope, RepoMapping, RepoMappingSummary,
};
use crate::services::{repo_detection_service, repo_mapping_service};
use crate::state::AppState;

fn build_summary(mapping: &RepoMapping, profile_name: String) -> RepoMappingSummary {
    RepoMappingSummary {
        id: mapping.id.clone(),
        repo_path: mapping.repo_path.to_string_lossy().to_string(),
        repo_name: mapping.repo_name.clone(),
        profile_id: mapping.profile_id.clone(),
        profile_name,
        git_config_scope: mapping.git_config_scope.clone(),
    }
}

#[tauri::command]
pub fn get_repo_mappings(state: State<'_, AppState>) -> Result<Vec<RepoMappingSummary>, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.lock().unwrap();
    let summaries = inner
        .repo_mappings
        .iter()
        .map(|m| {
            let profile_name = inner
                .profiles
                .iter()
                .find(|p| p.id == m.profile_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            build_summary(m, profile_name)
        })
        .collect();
    Ok(summaries)
}

#[tauri::command]
pub fn get_repo_mappings_for_profile(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RepoMappingSummary>, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.lock().unwrap();
    let profile_name = inner
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let summaries = inner
        .repo_mappings
        .iter()
        .filter(|m| m.profile_id == profile_id)
        .map(|m| build_summary(m, profile_name.clone()))
        .collect();
    Ok(summaries)
}

#[tauri::command]
pub fn create_repo_mapping(
    input: CreateRepoMappingInput,
    state: State<'_, AppState>,
) -> Result<RepoMapping, MazeSshError> {
    ensure_unlocked(&state)?;
    let repo_path = PathBuf::from(&input.repo_path);

    // Validate path exists
    if !repo_path.exists() {
        return Err(MazeSshError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {}", input.repo_path),
        )));
    }

    // Find git root
    let git_root = repo_detection_service::find_git_root(&repo_path)
        .ok_or_else(|| MazeSshError::NotAGitRepo(repo_path.clone()))?;

    let mut inner = state.inner.lock().unwrap();

    // Validate profile exists
    if !inner.profiles.iter().any(|p| p.id == input.profile_id) {
        return Err(MazeSshError::ProfileNotFound(input.profile_id));
    }

    // Check for duplicate mapping
    let normalized = repo_detection_service::normalize_path(&git_root);
    let normalized_str = normalized.to_string_lossy().to_lowercase();
    if inner.repo_mappings.iter().any(|m| {
        repo_detection_service::normalize_path(&m.repo_path)
            .to_string_lossy()
            .to_lowercase()
            == normalized_str
    }) {
        return Err(MazeSshError::DuplicateMapping(
            git_root.to_string_lossy().to_string(),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let repo_name = repo_detection_service::repo_name_from_path(&git_root);

    let mapping = RepoMapping {
        id: uuid::Uuid::new_v4().to_string(),
        repo_path: normalized,
        repo_name,
        profile_id: input.profile_id,
        git_config_scope: input.git_config_scope,
        created_at: now.clone(),
        updated_at: now,
    };

    inner.repo_mappings.push(mapping.clone());
    repo_mapping_service::save_mappings(&inner.repo_mappings)?;

    Ok(mapping)
}

#[tauri::command]
pub fn delete_repo_mapping(id: String, state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let mut inner = state.inner.lock().unwrap();
    let idx = inner
        .repo_mappings
        .iter()
        .position(|m| m.id == id)
        .ok_or_else(|| MazeSshError::RepoMappingNotFound(id))?;

    inner.repo_mappings.remove(idx);
    repo_mapping_service::save_mappings(&inner.repo_mappings)?;
    Ok(())
}

#[tauri::command]
pub fn update_repo_mapping_scope(
    id: String,
    scope: GitConfigScope,
    state: State<'_, AppState>,
) -> Result<RepoMapping, MazeSshError> {
    ensure_unlocked(&state)?;
    let mut inner = state.inner.lock().unwrap();
    let mapping = inner
        .repo_mappings
        .iter_mut()
        .find(|m| m.id == id)
        .ok_or_else(|| MazeSshError::RepoMappingNotFound(id))?;

    mapping.git_config_scope = scope;
    mapping.updated_at = chrono::Utc::now().to_rfc3339();

    let updated = mapping.clone();
    repo_mapping_service::save_mappings(&inner.repo_mappings)?;
    Ok(updated)
}
