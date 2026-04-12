use std::path::Path;
use std::process::Command;

use crate::error::MazeSshError;
use crate::models::repo_mapping::GitIdentityInfo;

fn hidden_cmd(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

fn find_git_binary() -> String {
    let candidates = [
        "C:\\Program Files\\Git\\cmd\\git.exe",
        "C:\\Program Files (x86)\\Git\\cmd\\git.exe",
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    "git".to_string()
}

pub fn set_git_identity_global(name: &str, email: &str) -> Result<(), MazeSshError> {
    let git = find_git_binary();

    let name_out = hidden_cmd(&git)
        .args(["config", "--global", "user.name", name])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(format!("Failed to run git: {}", e)))?;

    if !name_out.status.success() {
        let err = String::from_utf8_lossy(&name_out.stderr);
        return Err(MazeSshError::GitConfigError(format!(
            "git config user.name failed: {}",
            err
        )));
    }

    let email_out = hidden_cmd(&git)
        .args(["config", "--global", "user.email", email])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(format!("Failed to run git: {}", e)))?;

    if !email_out.status.success() {
        let err = String::from_utf8_lossy(&email_out.stderr);
        return Err(MazeSshError::GitConfigError(format!(
            "git config user.email failed: {}",
            err
        )));
    }

    Ok(())
}

pub fn set_git_identity_local(
    repo_path: &Path,
    name: &str,
    email: &str,
) -> Result<(), MazeSshError> {
    let git = find_git_binary();
    let repo = repo_path.to_string_lossy();

    let name_out = hidden_cmd(&git)
        .args(["-C", &repo, "config", "user.name", name])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(format!("Failed to run git: {}", e)))?;

    if !name_out.status.success() {
        let err = String::from_utf8_lossy(&name_out.stderr);
        return Err(MazeSshError::GitConfigError(format!(
            "git config user.name failed: {}",
            err
        )));
    }

    let email_out = hidden_cmd(&git)
        .args(["-C", &repo, "config", "user.email", email])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(format!("Failed to run git: {}", e)))?;

    if !email_out.status.success() {
        let err = String::from_utf8_lossy(&email_out.stderr);
        return Err(MazeSshError::GitConfigError(format!(
            "git config user.email failed: {}",
            err
        )));
    }

    Ok(())
}

pub fn get_git_identity_global() -> Result<GitIdentityInfo, MazeSshError> {
    let git = find_git_binary();

    let name = hidden_cmd(&git)
        .args(["config", "--global", "user.name"])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(e.to_string()))?;

    let email = hidden_cmd(&git)
        .args(["config", "--global", "user.email"])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(e.to_string()))?;

    Ok(GitIdentityInfo {
        user_name: String::from_utf8_lossy(&name.stdout).trim().to_string(),
        user_email: String::from_utf8_lossy(&email.stdout).trim().to_string(),
        scope: "global".to_string(),
    })
}

pub fn get_git_identity_local(repo_path: &Path) -> Result<GitIdentityInfo, MazeSshError> {
    let git = find_git_binary();
    let repo = repo_path.to_string_lossy();

    let name = hidden_cmd(&git)
        .args(["-C", &repo, "config", "user.name"])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(e.to_string()))?;

    let email = hidden_cmd(&git)
        .args(["-C", &repo, "config", "user.email"])
        .output()
        .map_err(|e| MazeSshError::GitConfigError(e.to_string()))?;

    Ok(GitIdentityInfo {
        user_name: String::from_utf8_lossy(&name.stdout).trim().to_string(),
        user_email: String::from_utf8_lossy(&email.stdout).trim().to_string(),
        scope: "local".to_string(),
    })
}
