//! Error types for queue operations

use thiserror::Error;

/// Queue errors
#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Job execution failed: {0}")]
    JobFailed(String),

    #[error("Job serialization failed: {0}")]
    SerializationError(String),

    #[error("Job deserialization failed: {0}")]
    DeserializationError(String),

    #[error("Queue backend error: {0}")]
    BackendError(String),

    #[error("Job timeout after {0}s")]
    Timeout(u64),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Worker error: {0}")]
    WorkerError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type for queue operations
pub type QueueResult<T> = Result<T, QueueError>;
