use thiserror::Error;

/// Result type for event operations
pub type Result<T> = std::result::Result<T, EventError>;

/// Errors that can occur during event handling
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Listener error: {0}")]
    ListenerError(String),

    #[error("Event dispatch failed: {0}")]
    DispatchError(String),

    #[error("Event serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}
