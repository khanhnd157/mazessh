use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Message too short: need {need} bytes, got {got}")]
    TooShort { need: usize, got: usize },

    #[error("Unknown message type: {0}")]
    UnknownMessageType(u8),

    #[error("Invalid message format: {0}")]
    InvalidFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
