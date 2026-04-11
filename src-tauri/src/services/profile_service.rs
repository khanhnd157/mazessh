use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::profile::SshProfile;

fn data_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".maze-ssh")
}

fn profiles_path() -> PathBuf {
    data_dir().join("profiles.json")
}

pub fn ensure_data_dir() -> Result<(), MazeSshError> {
    let dir = data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

pub fn load_profiles() -> Result<Vec<SshProfile>, MazeSshError> {
    let path = profiles_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)?;
    let profiles: Vec<SshProfile> = serde_json::from_str(&content)?;
    Ok(profiles)
}

pub fn save_profiles(profiles: &[SshProfile]) -> Result<(), MazeSshError> {
    ensure_data_dir()?;
    let content = serde_json::to_string_pretty(profiles)?;
    fs::write(profiles_path(), content)?;
    Ok(())
}

pub fn load_active_profile_id() -> Result<Option<String>, MazeSshError> {
    let path = data_dir().join("active.txt");
    if !path.exists() {
        return Ok(None);
    }
    let id = fs::read_to_string(&path)?.trim().to_string();
    if id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(id))
    }
}

pub fn save_active_profile_id(id: Option<&str>) -> Result<(), MazeSshError> {
    ensure_data_dir()?;
    let path = data_dir().join("active.txt");
    match id {
        Some(id) => fs::write(&path, id)?,
        None => {
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
    }
    Ok(())
}
