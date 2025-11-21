//! Tokenable trait for models that can have tokens

use crate::{models, repository::TokenRepository, NewToken, PersonalAccessToken, SanctumError};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sea_orm::DatabaseConnection;

/// Trait for models that can issue personal access tokens
#[async_trait]
pub trait Tokenable: Send + Sync + Sized {
    /// Get the tokenable type name (e.g., "User")
    fn tokenable_type() -> &'static str;

    /// Get the tokenable ID
    fn tokenable_id(&self) -> i64;

    /// Create a new personal access token
    async fn create_token(
        &self,
        name: &str,
        abilities: Vec<&str>,
        expires_at: Option<DateTime<Utc>>,
        db: &DatabaseConnection,
    ) -> Result<NewToken, SanctumError> {
        let repo = TokenRepository::new(db);
        repo.create(
            Self::tokenable_type(),
            self.tokenable_id(),
            name,
            abilities.iter().map(|s| s.to_string()).collect(),
            expires_at,
        )
        .await
    }

    /// Create a token with expiration in hours
    async fn create_token_with_expiry(
        &self,
        name: &str,
        abilities: Vec<&str>,
        hours: i64,
        db: &DatabaseConnection,
    ) -> Result<NewToken, SanctumError> {
        let expires_at = Utc::now() + Duration::hours(hours);
        self.create_token(name, abilities, Some(expires_at), db).await
    }

    /// Get all tokens for this model
    async fn tokens(&self, db: &DatabaseConnection) -> Result<Vec<PersonalAccessToken>, SanctumError> {
        let repo = TokenRepository::new(db);
        let models = repo
            .find_by_tokenable(Self::tokenable_type(), self.tokenable_id())
            .await?;

        Ok(models
            .into_iter()
            .map(|m| PersonalAccessToken::from_model(m))
            .collect())
    }

    /// Revoke all tokens
    async fn revoke_all_tokens(&self, db: &DatabaseConnection) -> Result<(), SanctumError> {
        let repo = TokenRepository::new(db);
        repo.revoke_all(Self::tokenable_type(), self.tokenable_id())
            .await
    }

    /// Revoke a specific token
    async fn revoke_token(&self, token_id: i64, db: &DatabaseConnection) -> Result<(), SanctumError> {
        let repo = TokenRepository::new(db);
        repo.revoke(token_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestUser {
        id: i64,
    }

    #[async_trait]
    impl Tokenable for TestUser {
        fn tokenable_type() -> &'static str {
            "User"
        }

        fn tokenable_id(&self) -> i64 {
            self.id
        }
    }

    #[tokio::test]
    async fn test_tokenable_type() {
        assert_eq!(TestUser::tokenable_type(), "User");
    }
}
