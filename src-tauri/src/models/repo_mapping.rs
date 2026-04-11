use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GitConfigScope {
    Local,
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMapping {
    pub id: String,
    pub repo_path: PathBuf,
    pub repo_name: String,
    pub profile_id: String,
    pub git_config_scope: GitConfigScope,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepoMappingInput {
    pub repo_path: String,
    pub profile_id: String,
    pub git_config_scope: GitConfigScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMappingSummary {
    pub id: String,
    pub repo_path: String,
    pub repo_name: String,
    pub profile_id: String,
    pub profile_name: String,
    pub git_config_scope: GitConfigScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitIdentityInfo {
    pub user_name: String,
    pub user_email: String,
    pub scope: String,
}
