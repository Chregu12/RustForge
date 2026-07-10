//! Sanctum error types

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SanctumError {
    #[error("Unauthenticated")]
    Unauthenticated,

    #[error("Missing authentication token")]
    MissingToken,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Database not configured")]
    DatabaseNotConfigured,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
}

impl IntoResponse for SanctumError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            SanctumError::Unauthenticated => (
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Unauthenticated".to_string(),
            ),
            SanctumError::MissingToken => (
                StatusCode::UNAUTHORIZED,
                "missing_token",
                "Missing authentication token".to_string(),
            ),
            SanctumError::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_token",
                "Invalid token".to_string(),
            ),
            SanctumError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                "token_expired",
                "Token expired".to_string(),
            ),
            SanctumError::InsufficientPermissions(p) => (
                StatusCode::FORBIDDEN,
                "insufficient_permissions",
                format!("Insufficient permissions: {}", p),
            ),
            SanctumError::DatabaseNotConfigured => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_not_configured",
                "Database not configured".to_string(),
            ),
            SanctumError::DatabaseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Internal server error".to_string(),
            ),
        };

        (status, Json(json!({ "error": error_code, "message": message }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sanctum_error_into_response_is_json() {
        let err = SanctumError::InvalidToken;
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let content_type = response
            .headers()
            .get("content-type")
            .expect("content-type header must be present");
        assert!(
            content_type.to_str().unwrap().contains("application/json"),
            "content-type must be application/json, got: {:?}",
            content_type
        );

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["error"], "invalid_token");
        assert_eq!(body["message"], "Invalid token");
    }

    #[tokio::test]
    async fn test_forbidden_error_is_json_403() {
        let err = SanctumError::InsufficientPermissions("admin".to_string());
        let response = err.into_response();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let content_type = response
            .headers()
            .get("content-type")
            .expect("content-type must be present");
        assert!(content_type.to_str().unwrap().contains("application/json"));

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["error"], "insufficient_permissions");
        assert!(body["message"].as_str().unwrap().contains("admin"));
    }
}
