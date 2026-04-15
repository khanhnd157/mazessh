use thiserror::Error;

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Vault not initialized at {0}")]
    NotInitialized(String),

    #[error("Vault already initialized")]
    AlreadyInitialized,

    #[error("Vault is locked (no active session)")]
    Locked,

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Duplicate key name: {0}")]
    DuplicateKeyName(String),

    #[error("Invalid passphrase")]
    InvalidPassphrase,

    #[error("Key generation error: {0}")]
    KeyGenError(String),

    #[error("Key parse error: {0}")]
    KeyParseError(String),

    #[error("Export denied: key policy forbids private export")]
    ExportDenied,

    #[error("Crypto error: {0}")]
    Crypto(#[from] maze_crypto::CryptoError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
