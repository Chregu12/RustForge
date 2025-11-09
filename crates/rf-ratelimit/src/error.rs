//! Error types for rate limiting

use thiserror::Error;

/// Result type for rate limiting operations
pub type RateLimitResult<T> = Result<T, RateLimitError>;

/// Rate limiting error types
#[derive(Debug, Error)]
pub enum RateLimitError {
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Backend error
    #[error("Backend error: {0}")]
    BackendError(String),

    /// Other error
    #[error("Rate limit error: {0}")]
    Other(String),
}
