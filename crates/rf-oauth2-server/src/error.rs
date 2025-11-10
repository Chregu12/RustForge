//! OAuth2 errors

use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde::Serialize;
use thiserror::Error;

/// OAuth2 errors
#[derive(Debug, Error)]
pub enum OAuth2Error {
    #[error("Invalid client: {0}")]
    InvalidClient(String),

    #[error("Invalid grant: {0}")]
    InvalidGrant(String),

    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Unauthorized client")]
    UnauthorizedClient,

    #[error("Unsupported grant type: {0}")]
    UnsupportedGrantType(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Server error: {0}")]
    ServerError(String),
}

/// Result type for OAuth2 operations
pub type OAuth2Result<T> = Result<T, OAuth2Error>;

/// OAuth2 error response (RFC 6749)
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
}

impl IntoResponse for OAuth2Error {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self {
            OAuth2Error::InvalidClient(_) => (StatusCode::UNAUTHORIZED, "invalid_client"),
            OAuth2Error::InvalidGrant(_) => (StatusCode::BAD_REQUEST, "invalid_grant"),
            OAuth2Error::InvalidScope(_) => (StatusCode::BAD_REQUEST, "invalid_scope"),
            OAuth2Error::InvalidToken(_) => (StatusCode::UNAUTHORIZED, "invalid_token"),
            OAuth2Error::UnauthorizedClient => (StatusCode::UNAUTHORIZED, "unauthorized_client"),
            OAuth2Error::UnsupportedGrantType(_) => {
                (StatusCode::BAD_REQUEST, "unsupported_grant_type")
            }
            OAuth2Error::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request"),
            OAuth2Error::ServerError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "server_error"),
        };

        let body = Json(ErrorResponse {
            error: error_code.to_string(),
            error_description: self.to_string(),
        });

        (status, body).into_response()
    }
}
