//! Password Grant implementation (Resource Owner Password Credentials)

use crate::{
    client::ClientRepository,
    config::PassportConfig,
    errors::{PassportError, PassportResult},
    grants::authorization_code::TokenResponse,
    scope::ScopeRepository,
    token::TokenRepository,
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

/// Password grant token request
#[derive(Debug, Deserialize)]
pub struct PasswordGrantRequest {
    pub grant_type: String,
    pub username: String,
    pub password: String,
    pub scope: Option<String>,
    pub client_id: i64,
    pub client_secret: Option<String>,
}

/// Trait for verifying user credentials
/// Application must implement this trait
#[async_trait]
pub trait PasswordVerifier: Send + Sync {
    /// Verify user credentials and return user ID if valid
    async fn verify(&self, username: &str, password: &str) -> PassportResult<i64>;
}

/// Password Grant handler
pub struct PasswordGrant<'a> {
    db: &'a DatabaseConnection,
    config: &'a PassportConfig,
}

impl<'a> PasswordGrant<'a> {
    pub fn new(db: &'a DatabaseConnection, config: &'a PassportConfig) -> Self {
        Self { db, config }
    }

    /// Issue token using password credentials
    pub async fn issue_token<V: PasswordVerifier>(
        &self,
        request: PasswordGrantRequest,
        verifier: &V,
    ) -> PassportResult<TokenResponse> {
        // Validate grant is enabled
        if !self.config.enable_password_grant {
            return Err(PassportError::UnsupportedGrantType(
                "Password grant is disabled".to_string(),
            ));
        }

        // Validate grant type
        if request.grant_type != "password" {
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

        // Verify this is a password client
        if !client.password_client {
            return Err(PassportError::UnauthorizedClient(
                "Client is not authorized for password grant".to_string(),
            ));
        }

        // Verify user credentials
        let user_id = verifier
            .verify(&request.username, &request.password)
            .await?;

        // Parse and validate scopes
        let scopes = self.parse_scopes(&request.scope)?;

        // Create access token
        let token_repo = TokenRepository::new(self.db);
        let access_expires_at = Utc::now() + self.config.access_token_duration();

        let access_token = token_repo
            .create_access_token(
                Some(user_id),
                client.id,
                scopes,
                access_expires_at,
                None,
            )
            .await?;

        // Create refresh token if enabled
        let refresh_token = if self.config.enable_refresh_token_grant {
            let refresh_expires_at = Utc::now() + self.config.refresh_token_duration();
            let refresh = token_repo
                .create_refresh_token(access_token.id.clone(), refresh_expires_at)
                .await?;
            Some(refresh.id)
        } else {
            None
        };

        let scopes = access_token.get_scopes().join(" ");
        let token_id = access_token.id;

        Ok(TokenResponse {
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_lifetime,
            access_token: token_id,
            refresh_token,
            scope: Some(scopes),
        })
    }

    /// Parse scope string into Vec<String>
    fn parse_scopes(&self, scope_str: &Option<String>) -> PassportResult<Vec<String>> {
        let scopes = if let Some(s) = scope_str {
            s.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            self.config.default_scopes.clone()
        };

        // Validate scopes if scope repository is configured
        if ScopeRepository::count() > 0 {
            if let Err(invalid) = ScopeRepository::validate(&scopes) {
                return Err(PassportError::InvalidScope(format!(
                    "Invalid scopes: {}",
                    invalid.join(", ")
                )));
            }
        }

        Ok(scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock password verifier for testing
    struct MockVerifier;

    #[async_trait]
    impl PasswordVerifier for MockVerifier {
        async fn verify(&self, username: &str, password: &str) -> PassportResult<i64> {
            if username == "test" && password == "password" {
                Ok(1)
            } else {
                Err(PassportError::InvalidCredentials)
            }
        }
    }

    #[tokio::test]
    async fn test_mock_verifier_accepts_correct_credentials() {
        let verifier = MockVerifier;
        let result = verifier.verify("test", "password").await;
        assert!(result.is_ok(), "correct credentials must succeed");
        assert_eq!(result.unwrap(), 1, "returned user_id must be 1");
    }

    #[tokio::test]
    async fn test_mock_verifier_rejects_wrong_credentials() {
        let verifier = MockVerifier;
        let result = verifier.verify("test", "wrong").await;
        assert!(result.is_err(), "wrong password must be rejected");
    }
}
