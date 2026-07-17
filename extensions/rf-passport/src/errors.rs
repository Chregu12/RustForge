//! Error types for OAuth2 operations

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OAuth2 error types following RFC 6749
#[derive(Debug, Error)]
pub enum PassportError {
    /// Invalid request parameters
    #[error("invalid_request: {0}")]
    InvalidRequest(String),

    /// Invalid client credentials
    #[error("invalid_client: {0}")]
    InvalidClient(String),

    /// Invalid grant
    #[error("invalid_grant: {0}")]
    InvalidGrant(String),

    /// Unauthorized client
    #[error("unauthorized_client: {0}")]
    UnauthorizedClient(String),

    /// Unsupported grant type
    #[error("unsupported_grant_type: {0}")]
    UnsupportedGrantType(String),

    /// Invalid scope
    #[error("invalid_scope: {0}")]
    InvalidScope(String),

    /// Access denied
    #[error("access_denied: {0}")]
    AccessDenied(String),

    /// Token expired
    #[error("Token has expired")]
    TokenExpired,

    /// Token revoked
    #[error("Token has been revoked")]
    TokenRevoked,

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,

    /// PKCE verification failed
    #[error("PKCE verification failed: {0}")]
    PkceVerificationFailed(String),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    /// Internal server error
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Missing required parameter
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// Invalid redirect URI
    #[error("Invalid redirect URI")]
    InvalidRedirectUri,

    /// Client not found
    #[error("Client not found")]
    ClientNotFound,

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// OAuth2 error response format (RFC 6749)
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuth2ErrorResponse {
    pub error: String,
    pub error_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_uri: Option<String>,
}

impl PassportError {
    /// Convert error to OAuth2 error type string
    pub fn error_type(&self) -> String {
        match self {
            Self::InvalidRequest(_) => "invalid_request".to_string(),
            Self::InvalidClient(_) => "invalid_client".to_string(),
            Self::InvalidGrant(_) => "invalid_grant".to_string(),
            Self::UnauthorizedClient(_) => "unauthorized_client".to_string(),
            Self::UnsupportedGrantType(_) => "unsupported_grant_type".to_string(),
            Self::InvalidScope(_) => "invalid_scope".to_string(),
            Self::AccessDenied(_) => "access_denied".to_string(),
            Self::TokenExpired | Self::TokenRevoked | Self::InvalidToken => {
                "invalid_token".to_string()
            }
            Self::PkceVerificationFailed(_) => "invalid_grant".to_string(),
            Self::MissingParameter(_) => "invalid_request".to_string(),
            Self::InvalidRedirectUri => "invalid_request".to_string(),
            Self::ClientNotFound => "invalid_client".to_string(),
            Self::UserNotFound | Self::InvalidCredentials => "invalid_grant".to_string(),
            Self::DatabaseError(_) | Self::InternalError(_) | Self::ConfigurationError(_) => {
                "server_error".to_string()
            }
        }
    }

    /// Get HTTP status code for error
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidRequest(_)
            | Self::InvalidGrant(_)
            | Self::UnsupportedGrantType(_)
            | Self::InvalidScope(_)
            | Self::MissingParameter(_)
            | Self::InvalidRedirectUri
            | Self::PkceVerificationFailed(_) => StatusCode::BAD_REQUEST,

            Self::InvalidClient(_) | Self::UnauthorizedClient(_) | Self::ClientNotFound => {
                StatusCode::UNAUTHORIZED
            }

            Self::AccessDenied(_)
            | Self::TokenExpired
            | Self::TokenRevoked
            | Self::InvalidToken
            | Self::UserNotFound
            | Self::InvalidCredentials => StatusCode::FORBIDDEN,

            Self::DatabaseError(_) | Self::InternalError(_) | Self::ConfigurationError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    /// Convert to OAuth2 error response
    pub fn to_oauth2_response(&self) -> OAuth2ErrorResponse {
        OAuth2ErrorResponse {
            error: self.error_type(),
            error_description: self.to_string(),
            error_uri: None,
        }
    }
}

/// Implement IntoResponse for Axum integration
impl IntoResponse for PassportError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(self.to_oauth2_response());
        (status, body).into_response()
    }
}

/// Result type alias for Passport operations
pub type PassportResult<T> = Result<T, PassportError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let err = PassportError::InvalidRequest("test".to_string());
        assert_eq!(err.error_type(), "invalid_request");
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_oauth2_response() {
        let err = PassportError::InvalidClient("bad credentials".to_string());
        let response = err.to_oauth2_response();
        assert_eq!(response.error, "invalid_client");
    }
}
