//! Error types for storage operations

use thiserror::Error;

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage error types
#[derive(Debug, Error)]
pub enum StorageError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Other(String),
}
