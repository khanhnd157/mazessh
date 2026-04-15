use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    GitHub,
    GitLab,
    Gitea,
    Bitbucket,
    Custom(String),
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::GitHub => write!(f, "github"),
            Provider::GitLab => write!(f, "gitlab"),
            Provider::Gitea => write!(f, "gitea"),
            Provider::Bitbucket => write!(f, "bitbucket"),
            Provider::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshProfile {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    pub email: String,
    pub git_username: String,
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
    pub host_alias: String,
    pub hostname: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub ssh_user: Option<String>,
    pub has_passphrase: bool,
    pub created_at: String,
    pub updated_at: String,
    /// If this profile's key is managed by the vault, holds the vault key ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_key_id: Option<String>,
}

impl SshProfile {
    pub fn ssh_user_or_default(&self) -> &str {
        self.ssh_user.as_deref().unwrap_or("git")
    }

    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(22)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    pub email: String,
    pub is_active: bool,
}

impl ProfileSummary {
    pub fn from_profile(profile: &SshProfile, active_id: &Option<String>) -> Self {
        Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            provider: profile.provider.clone(),
            email: profile.email.clone(),
            is_active: active_id.as_ref() == Some(&profile.id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProfileInput {
    pub name: String,
    pub provider: Provider,
    pub email: String,
    pub git_username: String,
    pub private_key_path: String,
    pub host_alias: String,
    pub hostname: String,
    pub port: Option<u16>,
    pub ssh_user: Option<String>,
    pub has_passphrase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProfileInput {
    pub name: Option<String>,
    pub provider: Option<Provider>,
    pub email: Option<String>,
    pub git_username: Option<String>,
    pub host_alias: Option<String>,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub ssh_user: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedKey {
    pub private_key_path: String,
    pub public_key_path: String,
    pub key_type: String,
    pub comment: String,
}
