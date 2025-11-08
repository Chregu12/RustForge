/// Queue error types
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Job not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid job: {0}")]
    InvalidJob(String),

    #[error("Queue full: {0}")]
    QueueFull(String),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Queue error: {0}")]
    Other(String),
}

impl From<redis::RedisError> for QueueError {
    fn from(err: redis::RedisError) -> Self {
        QueueError::Redis(err.to_string())
    }
}

impl From<serde_json::Error> for QueueError {
    fn from(err: serde_json::Error) -> Self {
        QueueError::Serialization(err.to_string())
    }
}

impl From<deadpool_redis::PoolError> for QueueError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        QueueError::Connection(err.to_string())
    }
}

pub type QueueResult<T> = Result<T, QueueError>;
