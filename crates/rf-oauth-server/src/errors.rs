//! OAuth2 Error Types

use thiserror::Error;

/// OAuth2 Result Type
pub type OAuth2Result<T> = Result<T, OAuth2Error>;

/// OAuth2 Error
#[derive(Debug, Error)]
pub enum OAuth2Error {
    /// Invalid request error
    #[error("invalid_request: {0}")]
    InvalidRequest(String),

    /// Invalid client error
    #[error("invalid_client: {0}")]
    InvalidClient(String),

    /// Invalid grant error
    #[error("invalid_grant: {0}")]
    InvalidGrant(String),

    /// Unauthorized client error
    #[error("unauthorized_client: {0}")]
    UnauthorizedClient(String),

    /// Unsupported grant type error
    #[error("unsupported_grant_type: {0}")]
    UnsupportedGrantType(String),

    /// Invalid scope error
    #[error("invalid_scope: {0}")]
    InvalidScope(String),

    /// Access denied error
    #[error("access_denied: {0}")]
    AccessDenied(String),

    /// Server error
    #[error("server_error: {0}")]
    ServerError(String),

    /// Temporarily unavailable error
    #[error("temporarily_unavailable: {0}")]
    TemporarilyUnavailable(String),

    /// Token expired error
    #[error("token_expired: {0}")]
    TokenExpired(String),

    /// Token revoked error
    #[error("token_revoked: {0}")]
    TokenRevoked(String),

    /// JWT error
    #[error("jwt_error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// Database error
    #[error("database_error: {0}")]
    DatabaseError(String),

    /// Internal error
    #[error("internal_error: {0}")]
    InternalError(String),
}

impl OAuth2Error {
    /// Get error code for OAuth2 response
    pub fn error_code(&self) -> &str {
        match self {
            OAuth2Error::InvalidRequest(_) => "invalid_request",
            OAuth2Error::InvalidClient(_) => "invalid_client",
            OAuth2Error::InvalidGrant(_) => "invalid_grant",
            OAuth2Error::UnauthorizedClient(_) => "unauthorized_client",
            OAuth2Error::UnsupportedGrantType(_) => "unsupported_grant_type",
            OAuth2Error::InvalidScope(_) => "invalid_scope",
            OAuth2Error::AccessDenied(_) => "access_denied",
            OAuth2Error::ServerError(_) => "server_error",
            OAuth2Error::TemporarilyUnavailable(_) => "temporarily_unavailable",
            OAuth2Error::TokenExpired(_) => "token_expired",
            OAuth2Error::TokenRevoked(_) => "token_revoked",
            OAuth2Error::JwtError(_) => "server_error",
            OAuth2Error::DatabaseError(_) => "server_error",
            OAuth2Error::InternalError(_) => "server_error",
        }
    }

    /// Get HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            OAuth2Error::InvalidRequest(_) => 400,
            OAuth2Error::InvalidClient(_) => 401,
            OAuth2Error::InvalidGrant(_) => 400,
            OAuth2Error::UnauthorizedClient(_) => 401,
            OAuth2Error::UnsupportedGrantType(_) => 400,
            OAuth2Error::InvalidScope(_) => 400,
            OAuth2Error::AccessDenied(_) => 403,
            OAuth2Error::ServerError(_) => 500,
            OAuth2Error::TemporarilyUnavailable(_) => 503,
            OAuth2Error::TokenExpired(_) => 401,
            OAuth2Error::TokenRevoked(_) => 401,
            OAuth2Error::JwtError(_) => 500,
            OAuth2Error::DatabaseError(_) => 500,
            OAuth2Error::InternalError(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(
            OAuth2Error::InvalidRequest("test".to_string()).error_code(),
            "invalid_request"
        );
        assert_eq!(
            OAuth2Error::InvalidClient("test".to_string()).error_code(),
            "invalid_client"
        );
        assert_eq!(
            OAuth2Error::InvalidGrant("test".to_string()).error_code(),
            "invalid_grant"
        );
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            OAuth2Error::InvalidRequest("test".to_string()).status_code(),
            400
        );
        assert_eq!(
            OAuth2Error::InvalidClient("test".to_string()).status_code(),
            401
        );
        assert_eq!(
            OAuth2Error::ServerError("test".to_string()).status_code(),
            500
        );
    }
}
