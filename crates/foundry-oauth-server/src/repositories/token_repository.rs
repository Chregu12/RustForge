//! OAuth2 Token Repository - PostgreSQL Implementation
//!
//! Provides database-backed token storage for access tokens, refresh tokens,
//! authorization codes, and personal access tokens

use crate::errors::{OAuth2Error, OAuth2Result};
use crate::models::{AccessToken, AuthorizationCode, PersonalAccessToken, RefreshToken};
use async_trait::async_trait;
use chrono::Utc;
use serde_json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Token Repository Trait
#[async_trait]
pub trait TokenRepository: Send + Sync {
    // Access Tokens
    async fn store_access_token(&self, token: AccessToken) -> OAuth2Result<AccessToken>;
    async fn find_access_token(&self, token: &str) -> OAuth2Result<Option<AccessToken>>;
    async fn find_access_token_by_id(&self, id: Uuid) -> OAuth2Result<Option<AccessToken>>;
    async fn revoke_access_token(&self, id: Uuid) -> OAuth2Result<()>;
    async fn delete_access_token(&self, id: Uuid) -> OAuth2Result<()>;

    // Refresh Tokens
    async fn store_refresh_token(&self, token: RefreshToken) -> OAuth2Result<RefreshToken>;
    async fn find_refresh_token(&self, token: &str) -> OAuth2Result<Option<RefreshToken>>;
    async fn revoke_refresh_token(&self, id: Uuid) -> OAuth2Result<()>;
    async fn delete_refresh_token(&self, id: Uuid) -> OAuth2Result<()>;

    // Authorization Codes
    async fn store_authorization_code(&self, code: AuthorizationCode) -> OAuth2Result<AuthorizationCode>;
    async fn find_authorization_code(&self, code: &str) -> OAuth2Result<Option<AuthorizationCode>>;
    async fn revoke_authorization_code(&self, id: Uuid) -> OAuth2Result<()>;
    async fn delete_authorization_code(&self, id: Uuid) -> OAuth2Result<()>;

    // Personal Access Tokens
    async fn store_personal_access_token(&self, token: PersonalAccessToken) -> OAuth2Result<PersonalAccessToken>;
    async fn find_personal_access_token(&self, token: &str) -> OAuth2Result<Option<PersonalAccessToken>>;
    async fn find_personal_access_tokens_by_user(&self, user_id: Uuid) -> OAuth2Result<Vec<PersonalAccessToken>>;
    async fn revoke_personal_access_token(&self, id: Uuid) -> OAuth2Result<()>;
    async fn delete_personal_access_token(&self, id: Uuid) -> OAuth2Result<()>;
    async fn update_personal_access_token_last_used(&self, id: Uuid) -> OAuth2Result<()>;

    // Cleanup
    async fn delete_expired_tokens(&self) -> OAuth2Result<usize>;
}

/// PostgreSQL Token Repository
pub struct PostgresTokenRepository {
    pool: PgPool,
}

impl PostgresTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_access_token(&self, row: &sqlx::postgres::PgRow) -> OAuth2Result<AccessToken> {
        let scopes_json: String = row.try_get("scopes")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get scopes: {}", e)))?;

        let scopes: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse scopes: {}", e)))?;

        Ok(AccessToken {
            id: row.try_get("id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            client_id: row.try_get("client_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            user_id: row.try_get("user_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            scopes,
            revoked: row.try_get("revoked").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
        })
    }

    fn row_to_refresh_token(&self, row: &sqlx::postgres::PgRow) -> OAuth2Result<RefreshToken> {
        Ok(RefreshToken {
            id: row.try_get("id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            access_token_id: row.try_get("access_token_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            revoked: row.try_get("revoked").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
        })
    }

    fn row_to_authorization_code(&self, row: &sqlx::postgres::PgRow) -> OAuth2Result<AuthorizationCode> {
        let scopes_json: String = row.try_get("scopes")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get scopes: {}", e)))?;

        let scopes: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse scopes: {}", e)))?;

        Ok(AuthorizationCode {
            id: row.try_get("id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            client_id: row.try_get("client_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            user_id: row.try_get("user_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            code: row.try_get("code").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            redirect_uri: row.try_get("redirect_uri").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            scopes,
            code_challenge: row.try_get("code_challenge").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            code_challenge_method: row.try_get("code_challenge_method").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            revoked: row.try_get("revoked").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
        })
    }

    fn row_to_personal_access_token(&self, row: &sqlx::postgres::PgRow) -> OAuth2Result<PersonalAccessToken> {
        let scopes_json: String = row.try_get("scopes")
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to get scopes: {}", e)))?;

        let scopes: Vec<String> = serde_json::from_str(&scopes_json)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to parse scopes: {}", e)))?;

        Ok(PersonalAccessToken {
            id: row.try_get("id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            user_id: row.try_get("user_id").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            name: row.try_get("name").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            token: row.try_get("token").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            scopes,
            revoked: row.try_get("revoked").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            last_used_at: row.try_get("last_used_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            expires_at: row.try_get("expires_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
            created_at: row.try_get("created_at").map_err(|e| OAuth2Error::InternalError(e.to_string()))?,
        })
    }
}

#[async_trait]
impl TokenRepository for PostgresTokenRepository {
    async fn store_access_token(&self, token: AccessToken) -> OAuth2Result<AccessToken> {
        let scopes_json = serde_json::to_string(&token.scopes)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oauth_access_tokens (id, client_id, user_id, token, scopes, revoked, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(token.id)
        .bind(token.client_id)
        .bind(token.user_id)
        .bind(&token.token)
        .bind(scopes_json)
        .bind(token.revoked)
        .bind(token.expires_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to insert access token: {}", e)))?;

        Ok(token)
    }

    async fn find_access_token(&self, token: &str) -> OAuth2Result<Option<AccessToken>> {
        let row = sqlx::query(
            "SELECT id, client_id, user_id, token, scopes, revoked, expires_at, created_at
             FROM oauth_access_tokens
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_access_token(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_access_token_by_id(&self, id: Uuid) -> OAuth2Result<Option<AccessToken>> {
        let row = sqlx::query(
            "SELECT id, client_id, user_id, token, scopes, revoked, expires_at, created_at
             FROM oauth_access_tokens
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_access_token(&row)?)),
            None => Ok(None),
        }
    }

    async fn revoke_access_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("UPDATE oauth_access_tokens SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to revoke access token: {}", e)))?;

        Ok(())
    }

    async fn delete_access_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("DELETE FROM oauth_access_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete access token: {}", e)))?;

        Ok(())
    }

    async fn store_refresh_token(&self, token: RefreshToken) -> OAuth2Result<RefreshToken> {
        sqlx::query(
            "INSERT INTO oauth_refresh_tokens (id, access_token_id, token, revoked, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(token.id)
        .bind(token.access_token_id)
        .bind(&token.token)
        .bind(token.revoked)
        .bind(token.expires_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to insert refresh token: {}", e)))?;

        Ok(token)
    }

    async fn find_refresh_token(&self, token: &str) -> OAuth2Result<Option<RefreshToken>> {
        let row = sqlx::query(
            "SELECT id, access_token_id, token, revoked, expires_at, created_at
             FROM oauth_refresh_tokens
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_refresh_token(&row)?)),
            None => Ok(None),
        }
    }

    async fn revoke_refresh_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("UPDATE oauth_refresh_tokens SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to revoke refresh token: {}", e)))?;

        Ok(())
    }

    async fn delete_refresh_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("DELETE FROM oauth_refresh_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete refresh token: {}", e)))?;

        Ok(())
    }

    async fn store_authorization_code(&self, code: AuthorizationCode) -> OAuth2Result<AuthorizationCode> {
        let scopes_json = serde_json::to_string(&code.scopes)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oauth_authorization_codes (id, client_id, user_id, code, redirect_uri, scopes, code_challenge, code_challenge_method, revoked, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind(code.id)
        .bind(code.client_id)
        .bind(code.user_id)
        .bind(&code.code)
        .bind(&code.redirect_uri)
        .bind(scopes_json)
        .bind(&code.code_challenge)
        .bind(&code.code_challenge_method)
        .bind(code.revoked)
        .bind(code.expires_at)
        .bind(code.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to insert authorization code: {}", e)))?;

        Ok(code)
    }

    async fn find_authorization_code(&self, code: &str) -> OAuth2Result<Option<AuthorizationCode>> {
        let row = sqlx::query(
            "SELECT id, client_id, user_id, code, redirect_uri, scopes, code_challenge, code_challenge_method, revoked, expires_at, created_at
             FROM oauth_authorization_codes
             WHERE code = $1"
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_authorization_code(&row)?)),
            None => Ok(None),
        }
    }

    async fn revoke_authorization_code(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("UPDATE oauth_authorization_codes SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to revoke authorization code: {}", e)))?;

        Ok(())
    }

    async fn delete_authorization_code(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("DELETE FROM oauth_authorization_codes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete authorization code: {}", e)))?;

        Ok(())
    }

    async fn store_personal_access_token(&self, token: PersonalAccessToken) -> OAuth2Result<PersonalAccessToken> {
        let scopes_json = serde_json::to_string(&token.scopes)
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to serialize scopes: {}", e)))?;

        sqlx::query(
            "INSERT INTO oauth_personal_access_tokens (id, user_id, name, token, scopes, revoked, last_used_at, expires_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(token.id)
        .bind(token.user_id)
        .bind(&token.name)
        .bind(&token.token)
        .bind(scopes_json)
        .bind(token.revoked)
        .bind(token.last_used_at)
        .bind(token.expires_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Failed to insert personal access token: {}", e)))?;

        Ok(token)
    }

    async fn find_personal_access_token(&self, token: &str) -> OAuth2Result<Option<PersonalAccessToken>> {
        let row = sqlx::query(
            "SELECT id, user_id, name, token, scopes, revoked, last_used_at, expires_at, created_at
             FROM oauth_personal_access_tokens
             WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        match row {
            Some(row) => Ok(Some(self.row_to_personal_access_token(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_personal_access_tokens_by_user(&self, user_id: Uuid) -> OAuth2Result<Vec<PersonalAccessToken>> {
        let rows = sqlx::query(
            "SELECT id, user_id, name, token, scopes, revoked, last_used_at, expires_at, created_at
             FROM oauth_personal_access_tokens
             WHERE user_id = $1
             ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OAuth2Error::InternalError(format!("Database error: {}", e)))?;

        rows.iter()
            .map(|row| self.row_to_personal_access_token(row))
            .collect()
    }

    async fn revoke_personal_access_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("UPDATE oauth_personal_access_tokens SET revoked = true WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to revoke personal access token: {}", e)))?;

        Ok(())
    }

    async fn delete_personal_access_token(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("DELETE FROM oauth_personal_access_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete personal access token: {}", e)))?;

        Ok(())
    }

    async fn update_personal_access_token_last_used(&self, id: Uuid) -> OAuth2Result<()> {
        sqlx::query("UPDATE oauth_personal_access_tokens SET last_used_at = $1 WHERE id = $2")
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to update last_used_at: {}", e)))?;

        Ok(())
    }

    async fn delete_expired_tokens(&self) -> OAuth2Result<usize> {
        let now = Utc::now();

        let result = sqlx::query("DELETE FROM oauth_access_tokens WHERE expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete expired tokens: {}", e)))?;

        let count = result.rows_affected() as usize;

        // Refresh tokens are deleted via CASCADE
        // Authorization codes cleanup
        sqlx::query("DELETE FROM oauth_authorization_codes WHERE expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete expired codes: {}", e)))?;

        // Personal access tokens cleanup
        sqlx::query("DELETE FROM oauth_personal_access_tokens WHERE expires_at IS NOT NULL AND expires_at < $1")
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| OAuth2Error::InternalError(format!("Failed to delete expired PATs: {}", e)))?;

        Ok(count)
    }
}
