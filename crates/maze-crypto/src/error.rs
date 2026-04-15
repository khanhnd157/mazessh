use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("KDF error: {0}")]
    KdfError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}
