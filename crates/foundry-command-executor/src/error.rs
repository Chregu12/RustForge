//! Error types for command execution

use thiserror::Error;

/// Errors that can occur during command execution
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// Command not found in registry
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Invalid command arguments
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Command execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Output capture failed
    #[error("Output capture failed: {0}")]
    OutputCaptureFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Generic error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type for command execution operations
pub type ExecutionResult<T> = Result<T, ExecutionError>;

impl From<serde_json::Error> for ExecutionError {
    fn from(err: serde_json::Error) -> Self {
        ExecutionError::SerializationError(err.to_string())
    }
}

impl From<foundry_plugins::CommandError> for ExecutionError {
    fn from(err: foundry_plugins::CommandError) -> Self {
        ExecutionError::ExecutionFailed(err.to_string())
    }
}
