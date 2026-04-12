use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::security::SecuritySettings;

fn data_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".maze-ssh")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

pub fn load_settings() -> SecuritySettings {
    let path = settings_path();
    if !path.exists() {
        return SecuritySettings::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => SecuritySettings::default(),
    }
}

pub fn save_settings(settings: &SecuritySettings) -> Result<(), MazeSshError> {
    let dir = data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(settings_path(), content)?;
    Ok(())
}
