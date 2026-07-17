use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthorizationError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Action not allowed: {0}")]
    Forbidden(String),

    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Gate not found: {0}")]
    GateNotFound(String),

    #[error("Invalid ability: {0}")]
    InvalidAbility(String),
}

pub type AuthorizationResult<T> = Result<T, AuthorizationError>;
