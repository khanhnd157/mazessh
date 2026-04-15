use std::path::Path;
use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::services::config_engine::{self, ConfigBackup};
use crate::state::AppState;

#[tauri::command]
pub fn preview_ssh_config(state: State<'_, AppState>) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    Ok(config_engine::preview_config(&inner.profiles))
}

#[tauri::command]
pub fn write_ssh_config(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;
    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    config_engine::write_config(&inner.profiles)
}

#[tauri::command]
pub fn backup_ssh_config(state: State<'_, AppState>) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    config_engine::backup_config()
}

#[tauri::command]
pub fn list_config_backups(state: State<'_, AppState>) -> Result<Vec<ConfigBackup>, MazeSshError> {
    ensure_unlocked(&state)?;
    config_engine::list_backups()
}

#[tauri::command]
pub fn rollback_ssh_config(
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<(), MazeSshError> {
    ensure_unlocked(&state)?;

    // Restrict to files inside ~/.ssh/ that match the backup naming pattern
    let ssh_dir = dirs::home_dir()
        .ok_or_else(|| MazeSshError::ConfigError("Home directory not found".to_string()))?
        .join(".ssh");

    let candidate = Path::new(&backup_path);

    // Canonicalize to resolve any symlinks or traversal components
    let canonical = candidate.canonicalize().map_err(|_| {
        MazeSshError::ConfigError(format!("Backup path does not exist: {}", backup_path))
    })?;
    let canonical_ssh = ssh_dir.canonicalize().unwrap_or(ssh_dir);

    if !canonical.starts_with(&canonical_ssh) {
        return Err(MazeSshError::ConfigError(
            "Backup path must be inside ~/.ssh/".to_string(),
        ));
    }

    // Filename must match expected pattern: config.backup.<timestamp>
    let filename = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if !filename.starts_with("config.backup.") {
        return Err(MazeSshError::ConfigError(
            "Invalid backup filename — expected config.backup.<timestamp>".to_string(),
        ));
    }

    config_engine::rollback_config(canonical.to_string_lossy().as_ref())
}

#[tauri::command]
pub fn read_current_ssh_config(state: State<'_, AppState>) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;
    config_engine::read_current_config()
}
