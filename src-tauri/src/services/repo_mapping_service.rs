use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::repo_mapping::RepoMapping;

fn data_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".maze-ssh")
}

fn mappings_path() -> PathBuf {
    data_dir().join("repo_mappings.json")
}

pub fn load_mappings() -> Result<Vec<RepoMapping>, MazeSshError> {
    let path = mappings_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)?;
    let mappings: Vec<RepoMapping> = serde_json::from_str(&content)?;
    Ok(mappings)
}

pub fn save_mappings(mappings: &[RepoMapping]) -> Result<(), MazeSshError> {
    let dir = data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    let content = serde_json::to_string_pretty(mappings)?;
    crate::services::profile_service::atomic_write(&mappings_path(), &content)?;
    Ok(())
}
