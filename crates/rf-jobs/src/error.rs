//! Error types for job processing

use rf_core::error::AppError;
use thiserror::Error;

/// Job processing errors
#[derive(Debug, Error)]
pub enum JobError {
    /// Job execution failed
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    /// Job timeout
    #[error("Job timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Redis error
    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    /// Custom error
    #[error("Custom error: {0}")]
    Custom(String),
}

/// Queue manager errors
#[derive(Debug, Error)]
pub enum QueueError {
    /// Redis connection error
    #[error("Redis connection error: {0}")]
    ConnectionError(#[from] redis::RedisError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Job not found
    #[error("Job not found: {0}")]
    JobNotFound(uuid::Uuid),

    /// Queue not found
    #[error("Queue not found: {0}")]
    QueueNotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Worker errors
#[derive(Debug, Error)]
pub enum WorkerError {
    /// Queue error
    #[error("Queue error: {0}")]
    QueueError(#[from] QueueError),

    /// Job error
    #[error("Job error: {0}")]
    JobError(#[from] JobError),

    /// Worker shutdown error
    #[error("Worker shutdown error: {0}")]
    ShutdownError(String),
}

/// Scheduler errors
#[derive(Debug, Error)]
pub enum SchedulerError {
    /// Invalid cron expression
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    /// Queue error
    #[error("Queue error: {0}")]
    QueueError(#[from] QueueError),
}

/// Convert JobError to AppError
impl From<JobError> for AppError {
    fn from(error: JobError) -> Self {
        AppError::Internal(error.into())
    }
}

/// Convert QueueError to AppError
impl From<QueueError> for AppError {
    fn from(error: QueueError) -> Self {
        AppError::Internal(error.into())
    }
}

/// Job result type
pub type JobResult = Result<(), JobError>;
