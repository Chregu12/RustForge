//! OAuth2 Client Repository - PostgreSQL Implementation
//!
//! Provides database-backed client storage using sqlx

use crate::clients::ClientRepository;
use crate::errors::{OAuth2Error, OAuth2Result};
use crate::models::Client;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// PostgreSQL Client Repository
pub struct PostgresClientRepository {
    pool: PgPool,
}

impl PostgresClientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Hash client secret using Argon2
    fn hash_secret(&self, secret: &str) -> OAuth2Result<String> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();

        argon2
            .hash_password(secret.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to hash secret: {}", e)))
    }

    /// Verify client secret
    fn verify_secret(&self, secret: &str, hash: &str) -> bool {
        let Ok(parsed_hash) = PasswordHash::new(hash) else {
            return false;
        };

        Argon2::default()
            .verify_password(secret.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Convert database row to Client
    fn row_to_client(&self, row: &sqlx::postgres::PgRow) -> OAuth2Result<Client> {
        let id: Uuid = row.try_get("id")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get id: {}", e)))?;

        let name: String = row.try_get("name")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get name: {}", e)))?;

        let secret_hash: Option<String> = row.try_get("secret_hash")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get secret_hash: {}", e)))?;

        let redirect_uris_json: String = row.try_get("redirect_uris")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get redirect_uris: {}", e)))?;

        let grants_json: String = row.try_get("grants")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get grants: {}", e)))?;

        let scopes_json: String = row.try_get("scopes")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get scopes: {}", e)))?;

        let revoked: bool = row.try_get("revoked")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get revoked: {}", e)))?;

        let created_at: DateTime<Utc> = row.try_get("created_at")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get created_at: {}", e)))?;

        let updated_at: DateTime<Utc> = row.try_get("updated_at")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get updated_at: {}", e)))?;

        let redirect_uris: Vec<String> = serde_json::from_str(&redirect_uris_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse redirect_uris: {}", e)))?;

        let grants: Vec<String> = serde_json::from_str(&grants_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse grants: {}", e)))?;

        let scopes: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse scopes: {}", e)))?;

        Ok(Client {
            id,
            name,
            secret: secret_hash.map(|_| "***REDACTED***".to_string()),
            redirect_uris,
            grants,
            scopes,
            revoked,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl ClientRepository for PostgresClientRepository {
    async fn find(&self, client_id: Uuid) -> OAuth2Result<Option<Client>> {
        let row = sqlx::query(
            "SELECT id, name, secret_hash, redirect_uris, grants, scopes, revoked, created_at, updated_at
             FROM oauth_clients
             WHERE id = $1"
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_client(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_credentials(
        &self,
        client_id: Uuid,
        client_secret: &str,
    ) -> OAuth2Result<Option<Client>> {
        let row = sqlx::query(
            "SELECT id, name, secret_hash, redirect_uris, grants, scopes, revoked, created_at, updated_at
             FROM oauth_clients
             WHERE id = $1"
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        let Some(row) = row else {
            return Ok(None);
        };

        let secret_hash: Option<String> = row.try_get("secret_hash")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get secret_hash: {}", e)))?;

        // Public clients don't have secrets
        let Some(stored_hash) = secret_hash else {
            return Err(OAuth2Error::InvalidClient(
                "Public client cannot authenticate with secret".to_string(),
            ));
        };

        // Verify secret
        if self.verify_secret(client_secret, &stored_hash) {
            Ok(Some(self.row_to_client(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn store(&self, client: Client) -> OAuth2Result<Client> {
        // Hash secret if confidential client
        let secret_hash = if let Some(secret) = &client.secret {
            if secret != "***REDACTED***" {
                Some(self.hash_secret(secret)?)
            } else {
                None
            }
        } else {
            None
        };

        let redirect_uris_json = serde_json::to_string(&client.redirect_uris)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize redirect_uris: {}", e)))?;

        let grants_json = serde_json::to_string(&client.grants)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize grants: {}", e)))?;

        let scopes_json = serde_json::to_string(&client.scopes)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oauth_clients (id, name, secret_hash, redirect_uris, grants, scopes, revoked, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(client.id)
        .bind(&client.name)
        .bind(&secret_hash)
        .bind(redirect_uris_json)
        .bind(grants_json)
        .bind(scopes_json)
        .bind(client.revoked)
        .bind(client.created_at)
        .bind(client.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to insert client: {}", e)))?;

        // Return client with redacted secret
        Ok(Client {
            secret: secret_hash.map(|_| "***REDACTED***".to_string()),
            ..client
        })
    }

    async fn update(&self, client: Client) -> OAuth2Result<Client> {
        let redirect_uris_json = serde_json::to_string(&client.redirect_uris)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize redirect_uris: {}", e)))?;

        let grants_json = serde_json::to_string(&client.grants)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize grants: {}", e)))?;

        let scopes_json = serde_json::to_string(&client.scopes)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize scopes: {}", e)))?;

        let result = sqlx::query(
            "UPDATE oauth_clients
             SET name = $2, redirect_uris = $3, grants = $4, scopes = $5, revoked = $6, updated_at = $7
             WHERE id = $1"
        )
        .bind(client.id)
        .bind(&client.name)
        .bind(redirect_uris_json)
        .bind(grants_json)
        .bind(scopes_json)
        .bind(client.revoked)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to update client: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(OAuth2Error::InvalidClient("Client not found".to_string()));
        }

        Ok(client)
    }

    async fn delete(&self, client_id: Uuid) -> OAuth2Result<()> {
        sqlx::query("DELETE FROM oauth_clients WHERE id = $1")
            .bind(client_id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete client: {}", e)))?;

        Ok(())
    }

    async fn revoke(&self, client_id: Uuid) -> OAuth2Result<()> {
        let result = sqlx::query(
            "UPDATE oauth_clients SET revoked = true, updated_at = $2 WHERE id = $1"
        )
        .bind(client_id)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to revoke client: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(OAuth2Error::InvalidClient("Client not found".to_string()));
        }

        Ok(())
    }

    async fn list(&self) -> OAuth2Result<Vec<Client>> {
        let rows = sqlx::query(
            "SELECT id, name, secret_hash, redirect_uris, grants, scopes, revoked, created_at, updated_at
             FROM oauth_clients
             ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to list clients: {}", e)))?;

        rows.iter()
            .map(|row| self.row_to_client(row))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> PgPool {
        // This would require a test database
        // For now, this is a placeholder
        todo!("Setup test database connection")
    }

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_store_and_find_client() {
        let pool = setup_test_db().await;
        let repo = PostgresClientRepository::new(pool);

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        let stored = repo.store(client).await.unwrap();
        assert_eq!(stored.secret, Some("***REDACTED***".to_string()));

        let found = repo.find(client_id).await.unwrap().unwrap();
        assert_eq!(found.name, "Test App");
    }
}
