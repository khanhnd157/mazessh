use std::process::Command;

use crate::models::profile::SshProfile;

/// Create a Command with hidden console window on Windows
pub fn hidden_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

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
    content.push_str(&format!(
        "# Active profile: {} ({})\n",
        profile.name, profile.provider
    ));
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

// ── SSH Agent integration (Windows) ──────────────────────────────────

/// Ensure the Windows OpenSSH Authentication Agent service is running.
/// Returns Ok(true) if agent is running, Ok(false) if failed to start.
pub fn ensure_agent_running() -> Result<bool, String> {
    // Check if agent is already running
    let status = hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Service ssh-agent).Status",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let status_str = String::from_utf8_lossy(&status.stdout).trim().to_string();
    if status_str == "Running" {
        return Ok(true);
    }

    // Try to start the service (requires admin for first-time, but if StartType=Manual it may work)
    let start = hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Start-Service ssh-agent",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !start.status.success() {
        // Try with elevated privileges
        let elevate = hidden_cmd("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Start-Process powershell -ArgumentList '-NoProfile -Command \"Set-Service ssh-agent -StartupType Manual; Start-Service ssh-agent\"' -Verb RunAs -Wait",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if !elevate.status.success() {
            return Ok(false);
        }
    }

    // Verify it started
    let verify = hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Service ssh-agent).Status",
        ])
        .output()
        .map_err(|e| e.to_string())?;

    let verify_str = String::from_utf8_lossy(&verify.stdout).trim().to_string();
    Ok(verify_str == "Running")
}

/// Clear all keys from agent, then add the specified key.
/// This ensures only one identity is active at a time.
pub fn agent_switch_key(key_path: &str) -> Result<String, String> {
    let ssh_add = find_ssh_add();

    // Remove all existing keys
    let _ = hidden_cmd(&ssh_add).arg("-D").output();

    // Add the new key
    let output = hidden_cmd(&ssh_add)
        .arg(key_path)
        .output()
        .map_err(|e| format!("Failed to run ssh-add: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    if output.status.success() || combined.contains("Identity added") {
        Ok(combined.trim().to_string())
    } else {
        Err(combined.trim().to_string())
    }
}

/// Remove all keys from agent
pub fn agent_clear_keys() -> Result<(), String> {
    let ssh_add = find_ssh_add();
    let _ = hidden_cmd(&ssh_add).arg("-D").output();
    Ok(())
}

/// List keys currently in agent
#[allow(dead_code)]
pub fn agent_list_keys() -> Result<String, String> {
    let ssh_add = find_ssh_add();
    let output = hidden_cmd(&ssh_add)
        .arg("-l")
        .output()
        .map_err(|e| format!("Failed to run ssh-add: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(stderr.trim().to_string())
    }
}

/// Set GIT_SSH_COMMAND as a persistent user environment variable on Windows
/// so all new terminal sessions pick it up.
pub fn set_user_env_git_ssh_command(profile: &SshProfile) -> Result<(), String> {
    let ssh_command = build_git_ssh_command(profile);

    hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "[Environment]::SetEnvironmentVariable('GIT_SSH_COMMAND', '{}', 'User')",
                ssh_command.replace("'", "''")
            ),
        ])
        .output()
        .map_err(|e| format!("Failed to set env: {}", e))?;

    Ok(())
}

/// Clear GIT_SSH_COMMAND from user environment
pub fn clear_user_env_git_ssh_command() -> Result<(), String> {
    hidden_cmd("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::SetEnvironmentVariable('GIT_SSH_COMMAND', $null, 'User')",
        ])
        .output()
        .map_err(|e| format!("Failed to clear env: {}", e))?;

    Ok(())
}

fn find_ssh_add() -> String {
    let system_path = std::path::Path::new("C:\\Windows\\System32\\OpenSSH\\ssh-add.exe");
    if system_path.exists() {
        system_path.to_string_lossy().to_string()
    } else {
        "ssh-add".to_string()
    }
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
