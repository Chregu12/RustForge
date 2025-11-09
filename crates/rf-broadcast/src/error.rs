//! Error types for broadcasting

use thiserror::Error;

/// Broadcast error types
#[derive(Debug, Error)]
pub enum BroadcastError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid channel: {0}")]
    InvalidChannel(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Broadcast result type
pub type BroadcastResult<T> = Result<T, BroadcastError>;
