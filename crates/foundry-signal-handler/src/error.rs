//! Error types for signal handling

use thiserror::Error;

/// Errors that can occur during signal handling
#[derive(Debug, Error)]
pub enum SignalError {
    /// Failed to register signal handler
    #[error("Failed to register signal handler: {0}")]
    RegistrationFailed(String),

    /// Signal handling failed
    #[error("Signal handling failed: {0}")]
    HandlingFailed(String),

    /// Callback execution failed
    #[error("Callback execution failed: {0}")]
    CallbackFailed(String),

    /// Shutdown failed
    #[error("Shutdown failed: {0}")]
    ShutdownFailed(String),

    /// Invalid signal
    #[error("Invalid signal: {0}")]
    InvalidSignal(String),

    /// Generic error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for signal handling operations
pub type SignalResult<T> = Result<T, SignalError>;

impl From<std::io::Error> for SignalError {
    fn from(err: std::io::Error) -> Self {
        SignalError::RegistrationFailed(err.to_string())
    }
}
