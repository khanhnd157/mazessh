use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::models::profile::DetectedKey;
use crate::services::key_scanner;
use crate::state::AppState;

#[tauri::command]
pub fn scan_ssh_keys(state: State<'_, AppState>) -> Result<Vec<DetectedKey>, MazeSshError> {
    ensure_unlocked(&state)?;
    key_scanner::scan_ssh_keys()
}
