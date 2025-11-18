//! Sanctum authentication extractor for Axum

use crate::{PersonalAccessToken, SanctumError, Tokenable};
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use sea_orm::DatabaseConnection;

/// Extractor for Sanctum-authenticated users
///
/// # Example
///
/// ```rust,ignore
/// async fn protected(SanctumAuth(user): SanctumAuth<User>) -> Json<User> {
///     Json(user)
/// }
/// ```
pub struct SanctumAuth<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for SanctumAuth<T>
where
    T: Tokenable + 'static,
    S: Send + Sync,
    DatabaseConnection: FromRequestParts<S>,
{
    type Rejection = SanctumError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract database connection
        let db = DatabaseConnection::from_request_parts(parts, state)
            .await
            .map_err(|_| SanctumError::Unauthenticated)?;

        // Extract bearer token from Authorization header
        let bearer_token = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(SanctumError::MissingToken)?;

        // Hash the token to match database
        let hashed = PersonalAccessToken::hash_token(bearer_token);

        // TODO: Find token in database
        // For now, return error
        Err(SanctumError::InvalidToken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctum_auth_compiles() {
        // Compilation test
        assert!(true);
    }
}
