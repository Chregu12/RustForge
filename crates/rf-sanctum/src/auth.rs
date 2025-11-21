//! Sanctum authentication extractor for Axum

use crate::{models, repository::TokenRepository, PersonalAccessToken, SanctumError};
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
pub struct SanctumAuth<T>(pub T, pub PersonalAccessToken);

/// Trait for loading a user from a token
#[async_trait]
pub trait LoadFromToken: Send + Sync + Sized {
    /// Load user by tokenable_id
    async fn load_from_token(
        tokenable_id: i64,
        db: &DatabaseConnection,
    ) -> Result<Self, SanctumError>;
}

#[async_trait]
impl<T, S> FromRequestParts<S> for SanctumAuth<T>
where
    T: LoadFromToken + 'static,
    S: Send + Sync,
{
    type Rejection = SanctumError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract database connection from extensions (must be added by middleware)
        let db = parts
            .extensions
            .get::<DatabaseConnection>()
            .ok_or(SanctumError::Unauthenticated)?;

        // Extract bearer token from Authorization header
        let bearer_token = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(SanctumError::MissingToken)?;

        // Hash the token to match database
        let hashed = PersonalAccessToken::hash_token(bearer_token);

        // Find token in database
        let repo = TokenRepository::new(db);
        let token_model = repo
            .find_by_token(&hashed)
            .await?
            .ok_or(SanctumError::InvalidToken)?;

        // Check if token is expired
        if token_model.is_expired() {
            return Err(SanctumError::TokenExpired);
        }

        // Update last_used_at
        repo.touch(token_model.id).await?;

        // Load user
        let user = T::load_from_token(token_model.tokenable_id, db).await?;

        // Convert model to PersonalAccessToken
        let token = PersonalAccessToken::from_model(token_model);

        // Store token in extensions for middleware to access
        parts.extensions.insert(token.clone());

        Ok(SanctumAuth(user, token))
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
