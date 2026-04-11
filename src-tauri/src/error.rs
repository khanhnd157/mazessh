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
}

impl Serialize for MazeSshError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
