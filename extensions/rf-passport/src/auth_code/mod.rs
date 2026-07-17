//! OAuth Authorization Code with PKCE support

pub mod model;
pub mod pkce;

pub use model::{ActiveModel, Column, Entity, Model, Relation};
pub use pkce::{
    generate_code_challenge, generate_code_verifier, verify_code_challenge, CodeChallengeMethod,
};

use crate::errors::{PassportError, PassportResult};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

/// Repository for OAuth Authorization Code operations
pub struct AuthCodeRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> AuthCodeRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new authorization code
    pub async fn create(
        &self,
        user_id: i64,
        client_id: i64,
        scopes: Vec<String>,
        redirect_uri: String,
        expires_at: DateTime<Utc>,
        code_challenge: Option<String>,
        code_challenge_method: Option<CodeChallengeMethod>,
    ) -> PassportResult<Model> {
        let code = Model::generate_code();

        let active_model = ActiveModel {
            id: Set(code),
            user_id: Set(user_id),
            client_id: Set(client_id),
            scopes: Set(json!(scopes)),
            revoked: Set(false),
            expires_at: Set(expires_at),
            code_challenge: Set(code_challenge),
            code_challenge_method: Set(code_challenge_method.map(|m| m.to_string())),
            redirect_uri: Set(redirect_uri),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;
        Ok(model)
    }

    /// Find authorization code by ID
    pub async fn find(&self, code: &str) -> PassportResult<Option<Model>> {
        let auth_code = Entity::find_by_id(code).one(self.db).await?;
        Ok(auth_code)
    }

    /// Find and validate authorization code
    pub async fn find_valid(&self, code: &str) -> PassportResult<Model> {
        let auth_code = self
            .find(code)
            .await?
            .ok_or(PassportError::InvalidGrant("Invalid authorization code".to_string()))?;

        if auth_code.revoked {
            return Err(PassportError::InvalidGrant(
                "Authorization code has been revoked".to_string(),
            ));
        }

        if auth_code.is_expired() {
            return Err(PassportError::InvalidGrant(
                "Authorization code has expired".to_string(),
            ));
        }

        Ok(auth_code)
    }

    /// Revoke an authorization code
    pub async fn revoke(&self, code: &str) -> PassportResult<()> {
        let auth_code = self
            .find(code)
            .await?
            .ok_or(PassportError::InvalidGrant("Invalid authorization code".to_string()))?;

        let mut active: ActiveModel = auth_code.into();
        active.revoked = Set(true);
        active.updated_at = Set(Utc::now());
        active.update(self.db).await?;

        Ok(())
    }

    /// Delete an authorization code
    pub async fn delete(&self, code: &str) -> PassportResult<()> {
        Entity::delete_by_id(code).exec(self.db).await?;
        Ok(())
    }

    /// Clean up expired authorization codes
    pub async fn cleanup_expired(&self) -> PassportResult<u64> {
        let result = Entity::delete_many()
            .filter(Column::ExpiresAt.lt(Utc::now()))
            .exec(self.db)
            .await?;

        Ok(result.rows_affected)
    }
}

// Auth-code flow requires a full grant pipeline with token storage;
// covered by integration tests.
