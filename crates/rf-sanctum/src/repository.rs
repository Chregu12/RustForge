//! Repository for managing Personal Access Tokens

use crate::{models, NewToken, PersonalAccessToken, SanctumError};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Repository for Personal Access Token operations
pub struct TokenRepository<'a> {
    db: &'a DatabaseConnection,
}

/// Token statistics for a tokenable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl<'a> TokenRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new personal access token
    pub async fn create(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
        name: &str,
        abilities: Vec<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<NewToken, SanctumError> {
        self.create_with_device(tokenable_type, tokenable_id, name, abilities, expires_at, None, None)
            .await
    }

    /// Create a new personal access token with device information
    pub async fn create_with_device(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
        name: &str,
        abilities: Vec<String>,
        expires_at: Option<DateTime<Utc>>,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<NewToken, SanctumError> {
        let plain_token = PersonalAccessToken::generate_token();
        let hashed_token = PersonalAccessToken::hash_token(&plain_token);

        let active_model = models::ActiveModel {
            tokenable_type: Set(tokenable_type.to_string()),
            tokenable_id: Set(tokenable_id),
            name: Set(name.to_string()),
            token: Set(hashed_token.clone()),
            abilities: Set(json!(abilities)),
            expires_at: Set(expires_at),
            user_agent: Set(user_agent),
            last_used_ip: Set(ip_address),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;

        Ok(NewToken {
            access_token: plain_token,
            token: PersonalAccessToken::from_model(model),
        })
    }

    /// Find a token by its hashed value
    pub async fn find_by_token(
        &self,
        hashed_token: &str,
    ) -> Result<Option<models::Model>, SanctumError> {
        let token = models::Entity::find()
            .filter(models::Column::Token.eq(hashed_token))
            .one(self.db)
            .await?;

        Ok(token)
    }

    /// Find all tokens for a tokenable
    pub async fn find_by_tokenable(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
    ) -> Result<Vec<models::Model>, SanctumError> {
        let tokens = models::Entity::find()
            .filter(models::Column::TokenableType.eq(tokenable_type))
            .filter(models::Column::TokenableId.eq(tokenable_id))
            .all(self.db)
            .await?;

        Ok(tokens)
    }

    /// Revoke a token by ID
    pub async fn revoke(&self, token_id: i64) -> Result<(), SanctumError> {
        models::Entity::delete_by_id(token_id).exec(self.db).await?;
        Ok(())
    }

    /// Revoke all tokens for a tokenable
    pub async fn revoke_all(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
    ) -> Result<(), SanctumError> {
        models::Entity::delete_many()
            .filter(models::Column::TokenableType.eq(tokenable_type))
            .filter(models::Column::TokenableId.eq(tokenable_id))
            .exec(self.db)
            .await?;
        Ok(())
    }

    /// Update last_used_at timestamp
    pub async fn touch(&self, token_id: i64) -> Result<(), SanctumError> {
        let token = models::Entity::find_by_id(token_id)
            .one(self.db)
            .await?
            .ok_or(SanctumError::InvalidToken)?;

        let mut active: models::ActiveModel = token.into();
        active.last_used_at = Set(Some(Utc::now()));
        active.update(self.db).await?;

        Ok(())
    }

    /// Update last_used_at timestamp and IP address
    pub async fn touch_with_ip(&self, token_id: i64, ip: Option<String>) -> Result<(), SanctumError> {
        let token = models::Entity::find_by_id(token_id)
            .one(self.db)
            .await?
            .ok_or(SanctumError::InvalidToken)?;

        let mut active: models::ActiveModel = token.into();
        active.last_used_at = Set(Some(Utc::now()));
        active.last_used_ip = Set(ip);
        active.update(self.db).await?;

        Ok(())
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) -> Result<u64, SanctumError> {
        let result = models::Entity::delete_many()
            .filter(models::Column::ExpiresAt.lt(Utc::now()))
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Prune expired tokens (alias for cleanup_expired)
    pub async fn prune_expired_tokens(&self) -> Result<u64, SanctumError> {
        self.cleanup_expired().await
    }

    /// Prune tokens older than specified days
    pub async fn prune_tokens_older_than(&self, days: u32) -> Result<u64, SanctumError> {
        let cutoff = Utc::now() - Duration::days(days as i64);

        let result = models::Entity::delete_many()
            .filter(models::Column::CreatedAt.lt(cutoff))
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Prune tokens not used in the last N days
    pub async fn prune_unused_tokens(&self, days: u32) -> Result<u64, SanctumError> {
        let cutoff = Utc::now() - Duration::days(days as i64);

        let result = models::Entity::delete_many()
            .filter(
                models::Column::LastUsedAt
                    .is_null()
                    .or(models::Column::LastUsedAt.lt(cutoff))
            )
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }

    /// Get tokens by IP address (for security audits)
    pub async fn find_by_ip(&self, ip: &str) -> Result<Vec<models::Model>, SanctumError> {
        let tokens = models::Entity::find()
            .filter(models::Column::LastUsedIp.eq(ip))
            .all(self.db)
            .await?;

        Ok(tokens)
    }

    /// Get token statistics for a tokenable
    pub async fn get_token_stats(
        &self,
        tokenable_type: &str,
        tokenable_id: i64,
    ) -> Result<TokenStats, SanctumError> {
        let all_tokens = self.find_by_tokenable(tokenable_type, tokenable_id).await?;

        let total = all_tokens.len();
        let expired = all_tokens.iter().filter(|t| t.is_expired()).count();
        let active = total - expired;

        let last_used = all_tokens
            .iter()
            .filter_map(|t| t.last_used_at)
            .max();

        Ok(TokenStats {
            total,
            active,
            expired,
            last_used_at: last_used,
        })
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
