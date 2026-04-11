use tauri::State;

use crate::error::MazeSshError;
use crate::services::config_engine;
use crate::state::AppState;

#[tauri::command]
pub fn preview_ssh_config(state: State<'_, AppState>) -> Result<String, MazeSshError> {
    let inner = state.inner.lock().unwrap();
    Ok(config_engine::preview_config(&inner.profiles))
}

#[tauri::command]
pub fn write_ssh_config(state: State<'_, AppState>) -> Result<(), MazeSshError> {
    let inner = state.inner.lock().unwrap();
    config_engine::write_config(&inner.profiles)
}

#[tauri::command]
pub fn backup_ssh_config() -> Result<String, MazeSshError> {
    config_engine::backup_config()
}
