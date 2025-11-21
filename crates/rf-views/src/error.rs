use thiserror::Error;

/// Errors that can occur when working with views
#[derive(Debug, Error)]
pub enum ViewError {
    /// Template was not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Error occurred during template rendering
    #[error("Render error: {0}")]
    RenderError(String),

    /// Error occurred during data serialization
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid context provided
    #[error("Invalid context")]
    InvalidContext,

    /// Template syntax error
    #[error("Template syntax error: {0}")]
    SyntaxError(String),

    /// Function registration error
    #[error("Function registration error: {0}")]
    FunctionError(String),

    /// Filter registration error
    #[error("Filter registration error: {0}")]
    FilterError(String),

    /// Component error
    #[error("Component error: {0}")]
    ComponentError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Tera error
    #[error("Tera error: {0}")]
    TeraError(#[from] tera::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type for view operations
pub type ViewResult<T> = Result<T, ViewError>;
