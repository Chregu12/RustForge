use thiserror::Error;

/// Result type for stub operations
pub type Result<T> = std::result::Result<T, StubError>;

/// Errors that can occur during stub operations
#[derive(Debug, Error)]
pub enum StubError {
    #[error("Stub not found: {0}")]
    StubNotFound(String),

    #[error("Template render error: {0}")]
    TemplateError(#[from] tera::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid stub format: {0}")]
    InvalidFormat(String),

    #[error("Variable substitution failed: {0}")]
    VariableError(String),

    #[error("Publishing failed: {0}")]
    PublishError(String),

    #[error("Other error: {0}")]
    Other(String),
}
