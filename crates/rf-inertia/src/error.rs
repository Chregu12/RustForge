//! Error types for Inertia.js operations

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

/// Result type for Inertia operations
pub type Result<T> = std::result::Result<T, InertiaError>;

/// Errors that can occur during Inertia operations
#[derive(Debug, Error)]
pub enum InertiaError {
    /// Serialization error
    #[error("Failed to serialize data: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Version mismatch error
    #[error("Asset version mismatch")]
    VersionMismatch,

    /// Invalid component name
    #[error("Invalid component name: {0}")]
    InvalidComponent(String),

    /// Missing required header
    #[error("Missing required header: {0}")]
    MissingHeader(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl IntoResponse for InertiaError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            InertiaError::SerializationError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
            InertiaError::VersionMismatch => (StatusCode::CONFLICT, self.to_string()),
            InertiaError::InvalidComponent(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            InertiaError::MissingHeader(ref msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            InertiaError::Other(ref msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        (
            status,
            Json(serde_json::json!({
                "error": message
            })),
        )
            .into_response()
    }
}
