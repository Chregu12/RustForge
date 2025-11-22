//! Repository for managing Personal Access Tokens

use crate::{models, NewToken, PersonalAccessToken, SanctumError};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

/// Repository for Personal Access Token operations
pub struct TokenRepository<'a> {
    db: &'a DatabaseConnection,
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
        let plain_token = PersonalAccessToken::generate_token();
        let hashed_token = PersonalAccessToken::hash_token(&plain_token);

        let active_model = models::ActiveModel {
            tokenable_type: Set(tokenable_type.to_string()),
            tokenable_id: Set(tokenable_id),
            name: Set(name.to_string()),
            token: Set(hashed_token.clone()),
            abilities: Set(json!(abilities)),
            expires_at: Set(expires_at),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;

        Ok(NewToken {
            access_token: plain_token,
            token: PersonalAccessToken {
                id: model.id,
                tokenable_type: model.tokenable_type,
                tokenable_id: model.tokenable_id,
                name: model.name,
                token: model.token,
                abilities,
                last_used_at: model.last_used_at,
                expires_at: model.expires_at,
                created_at: model.created_at,
                updated_at: model.updated_at,
            },
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

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) -> Result<u64, SanctumError> {
        let result = models::Entity::delete_many()
            .filter(models::Column::ExpiresAt.lt(Utc::now()))
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
