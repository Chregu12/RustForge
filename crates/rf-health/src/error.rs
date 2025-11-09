//! Error types for health checks

use thiserror::Error;

/// Health check errors
#[derive(Debug, Error)]
pub enum HealthError {
    #[error("Health check failed: {0}")]
    CheckFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for health checks
pub type HealthResult<T> = Result<T, HealthError>;
