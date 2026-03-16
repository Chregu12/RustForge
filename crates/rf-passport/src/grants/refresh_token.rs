//! Refresh Token Grant implementation

use crate::{
    client::ClientRepository,
    config::PassportConfig,
    errors::{PassportError, PassportResult},
    grants::authorization_code::TokenResponse,
    token::TokenRepository,
};
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

/// Refresh token grant request
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub grant_type: String,
    pub refresh_token: String,
    pub client_id: i64,
    pub client_secret: Option<String>,
    pub scope: Option<String>,
}

/// Refresh Token Grant handler
pub struct RefreshTokenGrant<'a> {
    db: &'a DatabaseConnection,
    config: &'a PassportConfig,
}

impl<'a> RefreshTokenGrant<'a> {
    pub fn new(db: &'a DatabaseConnection, config: &'a PassportConfig) -> Self {
        Self { db, config }
    }

    /// Issue new access token using refresh token
    pub async fn refresh(&self, request: RefreshTokenRequest) -> PassportResult<TokenResponse> {
        // Validate grant is enabled
        if !self.config.enable_refresh_token_grant {
            return Err(PassportError::UnsupportedGrantType(
                "Refresh token grant is disabled".to_string(),
            ));
        }

        // Validate grant type
        if request.grant_type != "refresh_token" {
            return Err(PassportError::InvalidRequest(
                "Invalid grant_type".to_string(),
            ));
        }

        // Find and validate client
        let client_repo = ClientRepository::new(self.db);
        let client = if let Some(secret) = &request.client_secret {
            client_repo
                .verify_credentials(request.client_id, secret)
                .await?
        } else {
            client_repo.find_active(request.client_id).await?
        };

        // Find and validate refresh token
        let token_repo = TokenRepository::new(self.db);
        let refresh_token = token_repo
            .find_valid_refresh_token(&request.refresh_token)
            .await?;

        // Find the original access token
        let original_access_token = token_repo
            .find_access_token(&refresh_token.access_token_id)
            .await?
            .ok_or(PassportError::InvalidToken)?;

        // Verify client ID matches
        if original_access_token.client_id != client.id {
            return Err(PassportError::InvalidGrant(
                "Client ID mismatch".to_string(),
            ));
        }

        // Determine scopes for new token
        let scopes = if let Some(scope_str) = &request.scope {
            // Requested scopes must be subset of original scopes
            let requested: Vec<String> = scope_str
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            for scope in &requested {
                if !original_access_token.has_scope(scope) {
                    return Err(PassportError::InvalidScope(format!(
                        "Scope '{}' was not in original token",
                        scope
                    )));
                }
            }

            requested
        } else {
            // Use same scopes as original token
            original_access_token.get_scopes()
        };

        // Revoke old refresh token (rotation)
        token_repo
            .revoke_refresh_token(&request.refresh_token)
            .await?;

        // Optionally revoke old access token
        if !original_access_token.revoked {
            token_repo
                .revoke_access_token(&original_access_token.id)
                .await?;
        }

        // Create new access token
        let access_expires_at = Utc::now() + self.config.access_token_duration();

        let access_token = token_repo
            .create_access_token(
                original_access_token.user_id,
                client.id,
                scopes,
                access_expires_at,
                None,
            )
            .await?;

        // Create new refresh token
        let refresh_expires_at = Utc::now() + self.config.refresh_token_duration();
        let new_refresh_token = token_repo
            .create_refresh_token(access_token.id.clone(), refresh_expires_at)
            .await?;

        let scopes = access_token.get_scopes().join(" ");
        let token_id = access_token.id;

        Ok(TokenResponse {
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime,
            access_token: token_id,
            refresh_token: Some(new_refresh_token.id),
            scope: Some(scopes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grant_compiles() {
        // Compilation test
        assert!(true);
    }
}
