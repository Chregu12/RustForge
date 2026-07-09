//! Error types for request handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rf_validation::ValidationErrors;
use serde_json::json;

/// Result type for request operations
pub type RequestResult<T> = Result<T, RequestError>;

/// Errors that can occur during request processing
#[derive(Debug)]
pub enum RequestError {
    /// Failed to parse request body
    InvalidBody(String),
    /// Request body exceeded the size limit (maps to 413 Payload Too Large)
    PayloadTooLarge(String),
    /// Field not found in request
    FieldNotFound(String),
    /// Field type mismatch
    InvalidFieldType(String),
    /// Validation failed
    ValidationFailed(ValidationErrors),
    /// User not authenticated
    Unauthenticated,
    /// Missing session
    NoSession,
    /// Generic request error
    Generic(String),
}

impl std::fmt::Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestError::InvalidBody(msg) => write!(f, "Invalid request body: {}", msg),
            RequestError::PayloadTooLarge(msg) => write!(f, "Payload too large: {}", msg),
            RequestError::FieldNotFound(field) => write!(f, "Field not found: {}", field),
            RequestError::InvalidFieldType(field) => write!(f, "Invalid field type: {}", field),
            RequestError::ValidationFailed(errors) => write!(f, "Validation failed: {:?}", errors),
            RequestError::Unauthenticated => write!(f, "User not authenticated"),
            RequestError::NoSession => write!(f, "No session available"),
            RequestError::Generic(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for RequestError {}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        match self {
            RequestError::InvalidBody(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({"error": msg}))).into_response()
            }
            RequestError::PayloadTooLarge(msg) => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({"error": msg})),
            )
                .into_response(),
            RequestError::FieldNotFound(field) => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Field '{}' not found", field)})),
            )
                .into_response(),
            RequestError::InvalidFieldType(field) => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("Invalid type for field '{}'", field)})),
            )
                .into_response(),
            RequestError::ValidationFailed(errors) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(errors)).into_response()
            }
            RequestError::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Authentication required"})),
            )
                .into_response(),
            RequestError::NoSession => (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "No session available"})),
            )
                .into_response(),
            RequestError::Generic(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": msg}))).into_response()
            }
        }
    }
}
