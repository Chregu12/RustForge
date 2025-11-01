use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    #[error("Service already registered: {0}")]
    ServiceAlreadyRegistered(String),

    #[error("Failed to resolve service: {0}")]
    ResolutionError(String),

    #[error("Type mismatch for service: {0}")]
    TypeMismatch(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Service locked by another thread")]
    ServiceLocked,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ContainerError>;
