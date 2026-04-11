use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Repo-to-profile mapping (M2 feature, struct defined early)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMapping {
    pub id: String,
    pub repo_path: PathBuf,
    pub profile_id: String,
}
