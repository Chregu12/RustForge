//! Error types for testing utilities

use thiserror::Error;

/// Testing error types
#[derive(Debug, Error)]
pub enum TestError {
    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Assertion failed: {0}")]
    AssertionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Test setup error: {0}")]
    SetupError(String),
}

/// Test result type
pub type TestResult<T> = Result<T, TestError>;
