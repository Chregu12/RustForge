//! Repository for OAuth Token operations

use super::access_token::{self, Entity as OAuthAccessToken};
use super::refresh_token::{self, Entity as OAuthRefreshToken};
use crate::errors::{PassportError, PassportResult};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

/// Repository for OAuth Token operations
pub struct TokenRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> TokenRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new access token
    pub async fn create_access_token(
        &self,
        user_id: Option<i64>,
        client_id: i64,
        scopes: Vec<String>,
        expires_at: DateTime<Utc>,
        name: Option<String>,
    ) -> PassportResult<access_token::Model> {
        let token_id = access_token::Model::generate_token_id();

        let active_model = access_token::ActiveModel {
            id: Set(token_id),
            user_id: Set(user_id),
            client_id: Set(client_id),
            name: Set(name),
            scopes: Set(json!(scopes)),
            revoked: Set(false),
            expires_at: Set(expires_at),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;
        Ok(model)
    }

    /// Create a new refresh token
    pub async fn create_refresh_token(
        &self,
        access_token_id: String,
        expires_at: DateTime<Utc>,
    ) -> PassportResult<refresh_token::Model> {
        let token_id = refresh_token::Model::generate_token_id();

        let active_model = refresh_token::ActiveModel {
            id: Set(token_id),
            access_token_id: Set(access_token_id),
            revoked: Set(false),
            expires_at: Set(expires_at),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;
        Ok(model)
    }

    /// Find access token by ID
    pub async fn find_access_token(
        &self,
        token_id: &str,
    ) -> PassportResult<Option<access_token::Model>> {
        let token = OAuthAccessToken::find_by_id(token_id)
            .one(self.db)
            .await?;
        Ok(token)
    }

    /// Find and validate access token
    pub async fn find_valid_access_token(
        &self,
        token_id: &str,
    ) -> PassportResult<access_token::Model> {
        let token = self
            .find_access_token(token_id)
            .await?
            .ok_or(PassportError::InvalidToken)?;

        if token.revoked {
            return Err(PassportError::TokenRevoked);
        }

        if token.is_expired() {
            return Err(PassportError::TokenExpired);
        }

        Ok(token)
    }

    /// Find refresh token by ID
    pub async fn find_refresh_token(
        &self,
        token_id: &str,
    ) -> PassportResult<Option<refresh_token::Model>> {
        let token = OAuthRefreshToken::find_by_id(token_id)
            .one(self.db)
            .await?;
        Ok(token)
    }

    /// Find and validate refresh token
    pub async fn find_valid_refresh_token(
        &self,
        token_id: &str,
    ) -> PassportResult<refresh_token::Model> {
        let token = self
            .find_refresh_token(token_id)
            .await?
            .ok_or(PassportError::InvalidToken)?;

        if token.revoked {
            return Err(PassportError::TokenRevoked);
        }

        if token.is_expired() {
            return Err(PassportError::TokenExpired);
        }

        Ok(token)
    }

    /// Find all access tokens for a user
    pub async fn find_tokens_by_user(
        &self,
        user_id: i64,
    ) -> PassportResult<Vec<access_token::Model>> {
        let tokens = OAuthAccessToken::find()
            .filter(access_token::Column::UserId.eq(user_id))
            .all(self.db)
            .await?;

        Ok(tokens)
    }

    /// Find all access tokens for a client
    pub async fn find_tokens_by_client(
        &self,
        client_id: i64,
    ) -> PassportResult<Vec<access_token::Model>> {
        let tokens = OAuthAccessToken::find()
            .filter(access_token::Column::ClientId.eq(client_id))
            .all(self.db)
            .await?;

        Ok(tokens)
    }

    /// Revoke an access token
    pub async fn revoke_access_token(&self, token_id: &str) -> PassportResult<()> {
        let token = self
            .find_access_token(token_id)
            .await?
            .ok_or(PassportError::InvalidToken)?;

        let mut active: access_token::ActiveModel = token.into();
        active.revoked = Set(true);
        active.updated_at = Set(Utc::now());
        active.update(self.db).await?;

        Ok(())
    }

    /// Revoke a refresh token
    pub async fn revoke_refresh_token(&self, token_id: &str) -> PassportResult<()> {
        let token = self
            .find_refresh_token(token_id)
            .await?
            .ok_or(PassportError::InvalidToken)?;

        let mut active: refresh_token::ActiveModel = token.into();
        active.revoked = Set(true);
        active.updated_at = Set(Utc::now());
        active.update(self.db).await?;

        Ok(())
    }

    /// Revoke all tokens for a user
    pub async fn revoke_all_user_tokens(&self, user_id: i64) -> PassportResult<u64> {
        let tokens = self.find_tokens_by_user(user_id).await?;

        let mut count = 0u64;
        for token in tokens {
            let mut active: access_token::ActiveModel = token.into();
            active.revoked = Set(true);
            active.updated_at = Set(Utc::now());
            active.update(self.db).await?;
            count += 1;
        }

        Ok(count)
    }

    /// Revoke all tokens for a client
    pub async fn revoke_all_client_tokens(&self, client_id: i64) -> PassportResult<u64> {
        let tokens = self.find_tokens_by_client(client_id).await?;

        let mut count = 0u64;
        for token in tokens {
            let mut active: access_token::ActiveModel = token.into();
            active.revoked = Set(true);
            active.updated_at = Set(Utc::now());
            active.update(self.db).await?;
            count += 1;
        }

        Ok(count)
    }

    /// Delete an access token
    pub async fn delete_access_token(&self, token_id: &str) -> PassportResult<()> {
        OAuthAccessToken::delete_by_id(token_id)
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// Delete a refresh token
    pub async fn delete_refresh_token(&self, token_id: &str) -> PassportResult<()> {
        OAuthRefreshToken::delete_by_id(token_id)
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// Clean up expired access tokens
    pub async fn cleanup_expired_access_tokens(&self) -> PassportResult<u64> {
        let result = OAuthAccessToken::delete_many()
            .filter(access_token::Column::ExpiresAt.lt(Utc::now()))
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Clean up expired refresh tokens
    pub async fn cleanup_expired_refresh_tokens(&self) -> PassportResult<u64> {
        let result = OAuthRefreshToken::delete_many()
            .filter(refresh_token::Column::ExpiresAt.lt(Utc::now()))
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_compiles() {
        // Compilation test
        assert!(true);
    }
}
