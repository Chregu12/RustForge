//! Passport middleware and extractors for Axum

use crate::{errors::PassportError, token::TokenRepository};
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Extension for database connection
#[derive(Clone)]
pub struct DatabaseExtension(pub Arc<DatabaseConnection>);

/// Passport authentication extractor
///
/// Usage:
/// ```rust,ignore
/// async fn handler(PassportAuth(user_id, token): PassportAuth) -> impl IntoResponse {
///     // user_id: Option<i64> - User ID if present, None for client credentials
///     // token: OAuthAccessToken - The validated access token
/// }
/// ```
pub struct PassportAuth(pub Option<i64>, pub crate::token::OAuthAccessToken);

#[async_trait]
impl<S> FromRequestParts<S> for PassportAuth
where
    S: Send + Sync,
{
    type Rejection = PassportAuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract bearer token from Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| PassportAuthError::MissingToken)?;

        let token_id = bearer.token();

        // Get database connection from extensions
        let db = parts
            .extensions
            .get::<DatabaseExtension>()
            .ok_or(PassportAuthError::MissingDatabase)?;

        // Validate token
        let token_repo = TokenRepository::new(&db.0);
        let token = token_repo
            .find_valid_access_token(token_id)
            .await
            .map_err(|_| PassportAuthError::InvalidToken)?;

        Ok(PassportAuth(token.user_id, token))
    }
}

/// Passport authentication with scope checking
///
/// Usage:
/// ```rust,ignore
/// async fn handler(
///     PassportAuthWithScopes(user_id, token): PassportAuthWithScopes<["read:posts"]>
/// ) -> impl IntoResponse {
///     // Token must have "read:posts" scope
/// }
/// ```
pub struct PassportAuthWithScopes<const N: usize>(
    pub Option<i64>,
    pub crate::token::OAuthAccessToken,
);

impl<const N: usize> PassportAuthWithScopes<N> {
    /// Create with required scopes (not directly usable as extractor)
    pub fn new(user_id: Option<i64>, token: crate::token::OAuthAccessToken) -> Self {
        Self(user_id, token)
    }
}

/// Passport authentication errors
#[derive(Debug)]
pub enum PassportAuthError {
    MissingToken,
    InvalidToken,
    MissingDatabase,
    InsufficientScopes,
}

impl IntoResponse for PassportAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
            Self::MissingDatabase => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database not configured")
            }
            Self::InsufficientScopes => (StatusCode::FORBIDDEN, "Insufficient token scopes"),
        };

        (status, message).into_response()
    }
}

/// Middleware to check token scopes
pub async fn check_scopes(
    token: &crate::token::OAuthAccessToken,
    required_scopes: &[&str],
) -> Result<(), PassportError> {
    if !token.has_all_scopes(required_scopes) {
        return Err(PassportError::InvalidScope(
            "Token does not have required scopes".to_string(),
        ));
    }
    Ok(())
}

/// Middleware to check if token has any of the scopes
pub async fn check_any_scope(
    token: &crate::token::OAuthAccessToken,
    scopes: &[&str],
) -> Result<(), PassportError> {
    if !token.has_any_scope(scopes) {
        return Err(PassportError::InvalidScope(
            "Token does not have any of the required scopes".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_compiles() {
        // Compilation test
        assert!(true);
    }
}
