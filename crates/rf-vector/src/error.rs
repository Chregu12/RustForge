use thiserror::Error;

/// Result type for vector operations.
pub type VectorResult<T> = Result<T, VectorError>;

/// Errors that can occur during vector operations.
#[derive(Debug, Error)]
pub enum VectorError {
    #[error("Document not found: {0}")]
    NotFound(String),

    #[error("Empty embedding: cannot operate on zero-dimensional vector")]
    EmptyEmbedding,

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Backend error: {0}")]
    BackendError(String),
}
