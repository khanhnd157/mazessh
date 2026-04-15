use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum MazeSshError {
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Key file not found: {0}")]
    KeyNotFound(PathBuf),

    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("SSH config error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("SSH connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Repo mapping not found: {0}")]
    RepoMappingNotFound(String),

    #[error("Not a git repository: {0}")]
    NotAGitRepo(PathBuf),

    #[error("Git config error: {0}")]
    GitConfigError(String),

    #[error("Duplicate mapping for repo: {0}")]
    DuplicateMapping(String),

    #[error("App is locked")]
    AppLocked,

    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("PIN is not configured")]
    PinNotSet,

    #[error("Internal state error")]
    StateLockError,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("WSL not available: {0}")]
    WslNotAvailable(String),

    #[error("WSL command failed: {0}")]
    WslCommandFailed(String),

    #[error("Bridge error: {0}")]
    BridgeError(String),
}

impl Serialize for MazeSshError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
