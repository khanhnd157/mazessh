use std::fs;
use std::path::PathBuf;
use tauri::State;

use crate::commands::security::ensure_unlocked;
use crate::error::MazeSshError;
use crate::services::{repo_detection_service, validation};
use crate::state::AppState;

/// Generate a pre-push git hook script that validates the SSH identity
#[tauri::command]
pub fn generate_git_hook(
    repo_path: String,
    state: State<'_, AppState>,
) -> Result<String, MazeSshError> {
    ensure_unlocked(&state)?;

    let p = PathBuf::from(&repo_path);
    let git_root = repo_detection_service::find_git_root(&p)
        .ok_or_else(|| MazeSshError::NotAGitRepo(p))?;

    let inner = state.inner.read().map_err(|_| MazeSshError::StateLockError)?;
    let mapping = repo_detection_service::lookup_mapping(&git_root, &inner.repo_mappings);

    let (profile_name, email) = match mapping {
        Some(m) => {
            let profile = inner.profiles.iter().find(|p| p.id == m.profile_id);
            match profile {
                Some(p) => (p.name.clone(), p.email.clone()),
                None => return Err(MazeSshError::ProfileNotFound(m.profile_id.clone())),
            }
        }
        None => {
            return Err(MazeSshError::ConfigError(
                "No mapping found for this repo. Create a repo mapping first.".to_string(),
            ))
        }
    };

    let safe_email = validation::shell_escape(&email);
    let safe_profile_name = validation::shell_escape(&profile_name);

    let hook_content = format!(
        r#"#!/bin/sh
# Maze SSH — pre-push identity validation hook
# Profile: {safe_profile_name}
# Expected email: {safe_email}

CURRENT_EMAIL=$(git config user.email)
EXPECTED_EMAIL='{safe_email}'

if [ "$CURRENT_EMAIL" != "$EXPECTED_EMAIL" ]; then
  echo ""
  echo "  [Maze SSH] Identity mismatch!"
  echo "  Expected: $EXPECTED_EMAIL ({safe_profile_name})"
  echo "  Current:  $CURRENT_EMAIL"
  echo ""
  echo "  Run: maze-ssh switch to fix, or set git config user.email"
  echo ""
  exit 1
fi
"#,
        safe_profile_name = safe_profile_name,
        safe_email = safe_email,
    );

    // Write to .git/hooks/pre-push
    let hooks_dir = git_root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("pre-push");
    fs::write(&hook_path, &hook_content)?;

    // Make executable (on Unix-like; on Windows this is a no-op but the script still works with git-bash)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(hook_path.to_string_lossy().to_string())
}

/// Remove the pre-push hook from a repo
#[tauri::command]
pub fn remove_git_hook(repo_path: String) -> Result<(), MazeSshError> {
    let p = PathBuf::from(&repo_path);
    let git_root = repo_detection_service::find_git_root(&p)
        .ok_or_else(|| MazeSshError::NotAGitRepo(p))?;

    let hook_path = git_root.join(".git").join("hooks").join("pre-push");
    if hook_path.exists() {
        // Only remove if it's our hook
        let content = fs::read_to_string(&hook_path)?;
        if content.contains("Maze SSH") {
            fs::remove_file(&hook_path)?;
        } else {
            return Err(MazeSshError::ConfigError(
                "Hook exists but was not created by Maze SSH".to_string(),
            ));
        }
    }
    Ok(())
}
