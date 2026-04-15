use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::security::SecuritySettings;

fn data_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".maze-ssh"))
}

fn settings_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("settings.json"))
}

pub fn load_settings() -> SecuritySettings {
    let path = match settings_path() {
        Some(p) => p,
        None => return SecuritySettings::default(),
    };
    if !path.exists() {
        return SecuritySettings::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SecuritySettings::default(),
    }
}

pub fn save_settings(settings: &SecuritySettings) -> Result<(), MazeSshError> {
    let dir = data_dir()
        .ok_or_else(|| MazeSshError::ConfigError("Home directory not found".to_string()))?;
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let path = settings_path()
        .ok_or_else(|| MazeSshError::ConfigError("Home directory not found".to_string()))?;
    let content = serde_json::to_string_pretty(settings)?;
    crate::services::profile_service::atomic_write(&path, &content)?;
    Ok(())
}
