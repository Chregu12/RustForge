use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("User cancelled the prompt")]
    Cancelled,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Dialoguer error: {0}")]
    DialoguerError(String),
}

pub type PromptResult<T> = Result<T, PromptError>;
