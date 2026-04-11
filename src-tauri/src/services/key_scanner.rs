use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::profile::DetectedKey;

pub fn get_ssh_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".ssh")
}

pub fn scan_ssh_keys() -> Result<Vec<DetectedKey>, MazeSshError> {
    let ssh_dir = get_ssh_dir();
    if !ssh_dir.exists() {
        return Ok(Vec::new());
    }

    let mut keys = Vec::new();
    let entries = fs::read_dir(&ssh_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip directories and .pub files
        if path.is_dir() || path.extension().map_or(false, |ext| ext == "pub") {
            continue;
        }

        // Skip known_hosts, config, authorized_keys, agent socket
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if matches!(
            filename.as_ref(),
            "known_hosts" | "known_hosts.old" | "config" | "authorized_keys" | "environment"
        ) {
            continue;
        }

        // Check if corresponding .pub file exists
        let pub_path = PathBuf::from(format!("{}.pub", path.display()));
        if !pub_path.exists() {
            continue;
        }

        // Detect key type from the public key
        let (key_type, comment) = parse_public_key(&pub_path);

        keys.push(DetectedKey {
            private_key_path: path.to_string_lossy().to_string(),
            public_key_path: pub_path.to_string_lossy().to_string(),
            key_type,
            comment,
        });
    }

    keys.sort_by(|a, b| a.private_key_path.cmp(&b.private_key_path));
    Ok(keys)
}

fn parse_public_key(pub_path: &PathBuf) -> (String, String) {
    let content = match fs::read_to_string(pub_path) {
        Ok(c) => c,
        Err(_) => return ("unknown".to_string(), String::new()),
    };

    let parts: Vec<&str> = content.trim().splitn(3, ' ').collect();
    let key_type = parts
        .first()
        .unwrap_or(&"unknown")
        .replace("ssh-", "")
        .to_string();
    let comment = parts.get(2).unwrap_or(&"").to_string();

    (key_type, comment)
}
