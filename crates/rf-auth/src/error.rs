//! Authentication and security errors

use rf_core::error::AppError;
use thiserror::Error;

/// Authentication errors
#[derive(Debug, Error)]
pub enum AuthError {
    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Token expired
    #[error("Token has expired")]
    TokenExpired,

    /// Password too weak
    #[error("Password too weak: {reason}")]
    WeakPassword { reason: String },

    /// Password hashing failed
    #[error("Password hashing failed")]
    HashingFailed {
        #[from]
        source: anyhow::Error,
    },

    /// JWT error
    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Email already exists
    #[error("Email already exists")]
    EmailExists,

    /// Invalid JWT secret
    #[error("JWT secret must be at least 32 characters")]
    InvalidSecret,

    /// Invalid bcrypt cost
    #[error("Bcrypt cost must be between 4 and 31")]
    InvalidBcryptCost,

    /// Token generation failed
    #[error("Token generation failed: {0}")]
    TokenGeneration(String),

    /// Email send failed
    #[error("Failed to send email: {0}")]
    EmailSendFailed(String),

    /// Email already verified
    #[error("Email is already verified")]
    AlreadyVerified,

    /// Token not found or invalid
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    /// Generic facade-level error message
    #[error("{0}")]
    Other(String),
}

/// Result type for auth operations
pub type AuthResult<T> = Result<T, AuthError>;

// Convert AuthError to AppError for HTTP responses
impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidCredentials | AuthError::TokenExpired => AppError::Unauthorized,

            AuthError::InvalidToken(msg) => AppError::BadRequest { message: msg },

            AuthError::WeakPassword { reason } => AppError::BadRequest { message: reason },

            AuthError::UserNotFound => AppError::NotFound {
                resource: "User".to_string(),
            },

            AuthError::EmailExists => AppError::Conflict {
                message: "Email already exists".to_string(),
            },

            AuthError::HashingFailed { source } => AppError::Internal(source),

            AuthError::InvalidSecret | AuthError::InvalidBcryptCost => {
                AppError::Internal(anyhow::anyhow!("Authentication configuration error"))
            }

            AuthError::JwtError(e) => AppError::Internal(e.into()),

            AuthError::TokenGeneration(msg) => {
                AppError::Internal(anyhow::anyhow!("Token generation failed: {}", msg))
            }

            AuthError::EmailSendFailed(msg) => {
                AppError::Internal(anyhow::anyhow!("Email send failed: {}", msg))
            }

            AuthError::AlreadyVerified => AppError::BadRequest {
                message: "Email is already verified".to_string(),
            },

            AuthError::Other(msg) => AppError::Internal(anyhow::anyhow!(msg)),
        }
    }
}
