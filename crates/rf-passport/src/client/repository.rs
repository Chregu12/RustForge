//! Repository for OAuth Client operations

use super::model::{self, Entity as OAuthClient};
use crate::errors::{PassportError, PassportResult};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::json;

/// Repository for OAuth Client operations
pub struct ClientRepository<'a> {
    db: &'a DatabaseConnection,
}

impl<'a> ClientRepository<'a> {
    pub fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }

    /// Create a new OAuth client
    pub async fn create(
        &self,
        user_id: Option<i64>,
        name: &str,
        redirect_uris: Vec<String>,
        personal_access_client: bool,
        password_client: bool,
        confidential: bool,
    ) -> PassportResult<(model::Model, Option<String>)> {
        let (secret_hash, plain_secret) = if confidential {
            let plain = model::Model::generate_secret();
            let hash = model::Model::hash_secret(&plain);
            (Some(hash), Some(plain))
        } else {
            (None, None)
        };

        let active_model = model::ActiveModel {
            user_id: Set(user_id),
            name: Set(name.to_string()),
            secret: Set(secret_hash),
            provider: Set(None),
            redirect: Set(json!(redirect_uris)),
            personal_access_client: Set(personal_access_client),
            password_client: Set(password_client),
            revoked: Set(false),
            ..Default::default()
        };

        let model = active_model.insert(self.db).await?;
        Ok((model, plain_secret))
    }

    /// Find client by ID
    pub async fn find_by_id(&self, client_id: i64) -> PassportResult<Option<model::Model>> {
        let client = OAuthClient::find_by_id(client_id).one(self.db).await?;
        Ok(client)
    }

    /// Find client by ID and verify it's not revoked
    pub async fn find_active(&self, client_id: i64) -> PassportResult<model::Model> {
        let client = OAuthClient::find_by_id(client_id)
            .one(self.db)
            .await?
            .ok_or(PassportError::ClientNotFound)?;

        if client.revoked {
            return Err(PassportError::InvalidClient("Client has been revoked".to_string()));
        }

        Ok(client)
    }

    /// Find all clients for a user
    pub async fn find_by_user(&self, user_id: i64) -> PassportResult<Vec<model::Model>> {
        let clients = OAuthClient::find()
            .filter(model::Column::UserId.eq(user_id))
            .all(self.db)
            .await?;

        Ok(clients)
    }

    /// Find personal access client for a user
    pub async fn find_personal_access_client(
        &self,
        user_id: i64,
    ) -> PassportResult<Option<model::Model>> {
        let client = OAuthClient::find()
            .filter(model::Column::UserId.eq(user_id))
            .filter(model::Column::PersonalAccessClient.eq(true))
            .filter(model::Column::Revoked.eq(false))
            .one(self.db)
            .await?;

        Ok(client)
    }

    /// Find password client
    pub async fn find_password_client(&self) -> PassportResult<Option<model::Model>> {
        let client = OAuthClient::find()
            .filter(model::Column::PasswordClient.eq(true))
            .filter(model::Column::Revoked.eq(false))
            .one(self.db)
            .await?;

        Ok(client)
    }

    /// Update client
    pub async fn update(&self, client: model::Model) -> PassportResult<model::Model> {
        let mut active: model::ActiveModel = client.into();
        active.updated_at = Set(chrono::Utc::now());
        let updated = active.update(self.db).await?;
        Ok(updated)
    }

    /// Revoke a client
    pub async fn revoke(&self, client_id: i64) -> PassportResult<()> {
        let client = self
            .find_by_id(client_id)
            .await?
            .ok_or(PassportError::ClientNotFound)?;

        let mut active: model::ActiveModel = client.into();
        active.revoked = Set(true);
        active.updated_at = Set(chrono::Utc::now());
        active.update(self.db).await?;

        Ok(())
    }

    /// Delete a client
    pub async fn delete(&self, client_id: i64) -> PassportResult<()> {
        OAuthClient::delete_by_id(client_id).exec(self.db).await?;
        Ok(())
    }

    /// Verify client credentials
    pub async fn verify_credentials(
        &self,
        client_id: i64,
        client_secret: &str,
    ) -> PassportResult<model::Model> {
        let client = self.find_active(client_id).await?;

        if !client.verify_secret(client_secret) {
            return Err(PassportError::InvalidClient(
                "Invalid client credentials".to_string(),
            ));
        }

        Ok(client)
    }

    /// Create or get personal access client for user
    pub async fn ensure_personal_access_client(
        &self,
        user_id: i64,
    ) -> PassportResult<model::Model> {
        // Try to find existing personal access client
        if let Some(client) = self.find_personal_access_client(user_id).await? {
            return Ok(client);
        }

        // Create new personal access client
        let (client, _) = self
            .create(
                Some(user_id),
                "Personal Access Client",
                vec![],
                true,
                false,
                false,
            )
            .await?;

        Ok(client)
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
