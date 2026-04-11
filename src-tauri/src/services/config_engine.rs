use std::fs;
use std::path::PathBuf;

use crate::error::MazeSshError;
use crate::models::profile::SshProfile;

const BEGIN_MARKER: &str = "# === BEGIN MAZE-SSH MANAGED ===";
const END_MARKER: &str = "# === END MAZE-SSH MANAGED ===";

fn ssh_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".ssh").join("config")
}

pub fn generate_config_block(profiles: &[SshProfile]) -> String {
    let mut config = String::new();
    config.push_str(BEGIN_MARKER);
    config.push('\n');
    config.push_str("# Do not edit this section manually. Managed by Maze SSH.\n");
    config.push('\n');

    for profile in profiles {
        config.push_str(&format!("Host {}\n", profile.host_alias));
        config.push_str(&format!("  HostName {}\n", profile.hostname));
        config.push_str(&format!("  User {}\n", profile.ssh_user_or_default()));
        config.push_str(&format!(
            "  IdentityFile {}\n",
            profile.private_key_path.to_string_lossy()
        ));
        config.push_str("  IdentitiesOnly yes\n");
        if let Some(port) = profile.port {
            if port != 22 {
                config.push_str(&format!("  Port {}\n", port));
            }
        }
        config.push('\n');
    }

    config.push_str(END_MARKER);
    config.push('\n');
    config
}

pub fn preview_config(profiles: &[SshProfile]) -> String {
    generate_config_block(profiles)
}

pub fn write_config(profiles: &[SshProfile]) -> Result<(), MazeSshError> {
    let config_path = ssh_config_path();
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };

    let new_block = generate_config_block(profiles);
    let updated = replace_managed_section(&existing, &new_block);

    // Ensure .ssh directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&config_path, updated)?;
    Ok(())
}

pub fn backup_config() -> Result<String, MazeSshError> {
    let config_path = ssh_config_path();
    if !config_path.exists() {
        return Err(MazeSshError::ConfigError(
            "No SSH config file to backup".to_string(),
        ));
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = config_path.with_file_name(format!("config.backup.{}", timestamp));
    fs::copy(&config_path, &backup_path)?;
    Ok(backup_path.to_string_lossy().to_string())
}

fn replace_managed_section(existing: &str, new_block: &str) -> String {
    if let (Some(begin), Some(end)) = (existing.find(BEGIN_MARKER), existing.find(END_MARKER)) {
        let end_of_marker = end + END_MARKER.len();
        // Skip newline after END_MARKER
        let end_pos = if existing[end_of_marker..].starts_with('\n') {
            end_of_marker + 1
        } else {
            end_of_marker
        };

        let mut result = String::new();
        result.push_str(&existing[..begin]);
        result.push_str(new_block);
        result.push_str(&existing[end_pos..]);
        result
    } else {
        // No existing managed section — append
        let mut result = existing.to_string();
        if !result.is_empty() && !result.ends_with('\n') {
            result.push('\n');
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(new_block);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_managed_section_no_existing() {
        let existing = "Host personal\n  HostName github.com\n";
        let new_block = "# === BEGIN MAZE-SSH MANAGED ===\nHost test\n# === END MAZE-SSH MANAGED ===\n";
        let result = replace_managed_section(existing, new_block);
        assert!(result.contains("Host personal"));
        assert!(result.contains("Host test"));
    }

    #[test]
    fn test_replace_managed_section_with_existing() {
        let existing = "Host personal\n  HostName github.com\n\n# === BEGIN MAZE-SSH MANAGED ===\nHost old\n# === END MAZE-SSH MANAGED ===\n\nHost other\n";
        let new_block = "# === BEGIN MAZE-SSH MANAGED ===\nHost new\n# === END MAZE-SSH MANAGED ===\n";
        let result = replace_managed_section(existing, new_block);
        assert!(result.contains("Host personal"));
        assert!(result.contains("Host new"));
        assert!(!result.contains("Host old"));
        assert!(result.contains("Host other"));
    }
}
