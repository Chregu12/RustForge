//! Personal Access Token support

use crate::{
    client::ClientRepository, config::PassportConfig, errors::PassportResult,
    token::TokenRepository,
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::DatabaseConnection;

/// Trait for models that can create personal access tokens
#[async_trait]
pub trait HasApiTokens {
    /// Get the user/model ID
    fn get_id(&self) -> i64;

    /// Create a personal access token
    async fn create_token(
        &self,
        name: &str,
        scopes: Vec<String>,
        db: &DatabaseConnection,
        config: &PassportConfig,
    ) -> PassportResult<String> {
        let user_id = self.get_id();

        // Ensure personal access client exists
        let client_repo = ClientRepository::new(db);
        let client = client_repo.ensure_personal_access_client(user_id).await?;

        // Create access token
        let token_repo = TokenRepository::new(db);

        let expires_at = if let Some(lifetime) = config.personal_access_token_duration() {
            Utc::now() + lifetime
        } else {
            // Far future expiration (100 years)
            Utc::now() + chrono::Duration::days(36500)
        };

        let access_token = token_repo
            .create_access_token(
                Some(user_id),
                client.id,
                scopes,
                expires_at,
                Some(name.to_string()),
            )
            .await?;

        Ok(access_token.id)
    }

    /// Get all personal access tokens
    async fn tokens(&self, db: &DatabaseConnection) -> PassportResult<Vec<crate::token::OAuthAccessToken>> {
        let user_id = self.get_id();
        let token_repo = TokenRepository::new(db);
        token_repo.find_tokens_by_user(user_id).await
    }

    /// Revoke a specific token
    async fn revoke_token(&self, token_id: &str, db: &DatabaseConnection) -> PassportResult<()> {
        let token_repo = TokenRepository::new(db);
        token_repo.revoke_access_token(token_id).await
    }

    /// Revoke all tokens
    async fn revoke_all_tokens(&self, db: &DatabaseConnection) -> PassportResult<u64> {
        let user_id = self.get_id();
        let token_repo = TokenRepository::new(db);
        token_repo.revoke_all_user_tokens(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example implementation
    struct User {
        id: i64,
    }

    #[async_trait]
    impl HasApiTokens for User {
        fn get_id(&self) -> i64 {
            self.id
        }
    }

    #[test]
    fn test_has_api_tokens_get_id_returns_correct_value() {
        let user = User { id: 42 };
        // HasApiTokens::get_id must return the struct's own id field.
        assert_eq!(user.get_id(), 42);
    }
}
