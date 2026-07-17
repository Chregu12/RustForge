//! Nightwatch error types

use thiserror::Error;

/// Result type for Nightwatch operations
pub type NightwatchResult<T> = Result<T, NightwatchError>;

/// Nightwatch error types
#[derive(Debug, Error)]
pub enum NightwatchError {
    /// Check failed
    #[error("Check failed: {0}")]
    CheckFailed(String),

    /// Alert failed
    #[error("Alert failed: {0}")]
    AlertFailed(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
