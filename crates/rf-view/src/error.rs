use thiserror::Error;

#[derive(Error, Debug)]
pub enum ViewError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Template rendering failed: {0}")]
    RenderError(String),

    #[error("Template initialization failed: {0}")]
    InitError(String),

    #[error("Invalid template data: {0}")]
    InvalidData(String),

    #[error("Tera error: {0}")]
    TeraError(#[from] tera::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type ViewResult<T> = Result<T, ViewError>;
