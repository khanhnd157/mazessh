use crate::models::profile::SshProfile;

pub fn build_git_ssh_command(profile: &SshProfile) -> String {
    let key_path = profile.private_key_path.to_string_lossy();
    let port = profile.port_or_default();

    if port == 22 {
        format!(
            "ssh -i \"{}\" -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
            key_path
        )
    } else {
        format!(
            "ssh -i \"{}\" -p {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
            key_path, port
        )
    }
}

pub fn build_env_file_content(profile: &SshProfile) -> String {
    let ssh_command = build_git_ssh_command(profile);
    let mut content = String::new();
    content.push_str(&format!("export GIT_SSH_COMMAND='{}'\n", ssh_command));
    content.push_str(&format!("# Active profile: {} ({})\n", profile.name, profile.provider));
    content
}

/// Write the env file to ~/.maze-ssh/env for shell sourcing
pub fn write_env_file(profile: &SshProfile) -> Result<(), std::io::Error> {
    let home = dirs::home_dir().expect("Could not find home directory");
    let env_path = home.join(".maze-ssh").join("env");
    let content = build_env_file_content(profile);
    std::fs::write(&env_path, content)?;
    Ok(())
}

/// Clear the env file when deactivating
pub fn clear_env_file() -> Result<(), std::io::Error> {
    let home = dirs::home_dir().expect("Could not find home directory");
    let env_path = home.join(".maze-ssh").join("env");
    if env_path.exists() {
        std::fs::write(&env_path, "# No active profile\n")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::profile::Provider;
    use std::path::PathBuf;

    fn test_profile(port: Option<u16>) -> SshProfile {
        SshProfile {
            id: "test-id".to_string(),
            name: "Test Profile".to_string(),
            provider: Provider::GitHub,
            email: "test@example.com".to_string(),
            git_username: "testuser".to_string(),
            private_key_path: PathBuf::from("/home/user/.ssh/id_ed25519"),
            public_key_path: PathBuf::from("/home/user/.ssh/id_ed25519.pub"),
            host_alias: "github-test".to_string(),
            hostname: "github.com".to_string(),
            port,
            ssh_user: None,
            has_passphrase: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_build_git_ssh_command_default_port() {
        let profile = test_profile(None);
        let cmd = build_git_ssh_command(&profile);
        assert!(cmd.contains("ssh -i"));
        assert!(cmd.contains("id_ed25519"));
        assert!(cmd.contains("IdentitiesOnly=yes"));
        assert!(!cmd.contains("-p "));
    }

    #[test]
    fn test_build_git_ssh_command_custom_port() {
        let profile = test_profile(Some(2222));
        let cmd = build_git_ssh_command(&profile);
        assert!(cmd.contains("-p 2222"));
    }

    #[test]
    fn test_build_env_file_content() {
        let profile = test_profile(None);
        let content = build_env_file_content(&profile);
        assert!(content.contains("export GIT_SSH_COMMAND="));
        assert!(content.contains("Active profile: Test Profile"));
    }
}
