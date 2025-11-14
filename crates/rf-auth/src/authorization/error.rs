//! Authorization error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Authorization error types
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    /// User is not authenticated
    #[error("Unauthorized")]
    Unauthorized,

    /// User is authenticated but lacks permission
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Policy not found for the given resource type
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    /// Gate not found
    #[error("Gate not found: {0}")]
    GateNotFound(String),

    /// Resource not found in request
    #[error("Resource not found in request")]
    ResourceNotFound,

    /// User not found in request
    #[error("User not found in request")]
    UserNotFound,
}

impl IntoResponse for AuthorizationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthorizationError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthorizationError::Forbidden(_) => (StatusCode::FORBIDDEN, self.to_string()),
            AuthorizationError::PolicyNotFound(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AuthorizationError::GateNotFound(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AuthorizationError::ResourceNotFound => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AuthorizationError::UserNotFound => (StatusCode::UNAUTHORIZED, self.to_string()),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

/// Result type for authorization operations
pub type AuthorizationResult<T> = Result<T, AuthorizationError>;
