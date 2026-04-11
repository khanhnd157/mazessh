use crate::error::MazeSshError;
use crate::models::profile::DetectedKey;
use crate::services::key_scanner;

#[tauri::command]
pub fn scan_ssh_keys() -> Result<Vec<DetectedKey>, MazeSshError> {
    key_scanner::scan_ssh_keys()
}
