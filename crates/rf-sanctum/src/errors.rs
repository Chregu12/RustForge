//! Sanctum error types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
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

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
}

impl IntoResponse for SanctumError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            SanctumError::Unauthenticated => (StatusCode::UNAUTHORIZED, self.to_string()),
            SanctumError::MissingToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            SanctumError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            SanctumError::TokenExpired => (StatusCode::UNAUTHORIZED, self.to_string()),
            SanctumError::InsufficientPermissions(_) => (StatusCode::FORBIDDEN, self.to_string()),
            SanctumError::DatabaseError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal error".to_string(),
            ),
        };

        (status, message).into_response()
    }
}
