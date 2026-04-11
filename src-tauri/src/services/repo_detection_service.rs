use std::path::{Path, PathBuf};

use crate::models::repo_mapping::RepoMapping;

/// Traverse up from `path` to find the git repository root (directory containing `.git/`).
pub fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return Some(normalize_path(&current));
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Normalize a path: canonicalize and strip Windows UNC prefix.
pub fn normalize_path(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Find a repo mapping matching the given git root path.
/// Case-insensitive comparison on Windows.
pub fn lookup_mapping<'a>(
    repo_path: &Path,
    mappings: &'a [RepoMapping],
) -> Option<&'a RepoMapping> {
    let normalized = normalize_path(repo_path);
    let normalized_str = normalized.to_string_lossy().to_lowercase();

    mappings.iter().find(|m| {
        let m_normalized = normalize_path(&m.repo_path);
        m_normalized.to_string_lossy().to_lowercase() == normalized_str
    })
}

/// Combined: find git root from any path, then look up its mapping.
#[allow(dead_code)]
pub fn detect_and_resolve<'a>(
    path: &Path,
    mappings: &'a [RepoMapping],
) -> Option<(PathBuf, &'a RepoMapping)> {
    let git_root = find_git_root(path)?;
    let mapping = lookup_mapping(&git_root, mappings)?;
    Some((git_root, mapping))
}

/// Extract repo name from path (last component).
pub fn repo_name_from_path(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_find_git_root() {
        let tmp = std::env::temp_dir().join("maze_ssh_test_git_root");
        let nested = tmp.join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(tmp.join(".git")).unwrap();

        let result = find_git_root(&nested);
        assert!(result.is_some());
        let root = result.unwrap();
        assert!(root.join(".git").exists());

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_find_git_root_none() {
        let tmp = std::env::temp_dir().join("maze_ssh_test_no_git");
        fs::create_dir_all(&tmp).unwrap();

        let result = find_git_root(&tmp);
        // May or may not find a .git above temp — just ensure no panic
        let _ = result;

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn test_repo_name_from_path() {
        let name = repo_name_from_path(Path::new("C:\\Users\\dev\\my-project"));
        assert_eq!(name, "my-project");
    }
}
