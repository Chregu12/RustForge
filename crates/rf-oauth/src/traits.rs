//! OAuth traits

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OAuth user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUser {
    pub provider: String,
    pub provider_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

/// OAuth tokens (access token and optional refresh token)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
}

/// OAuth provider trait
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider name (e.g., "google", "github")
    fn name(&self) -> &'static str;

    /// Get the authorization URL with state parameter
    fn authorize_url(&self, state: &str) -> String;

    /// Exchange authorization code for access token
    async fn exchange_code(&self, code: &str) -> crate::Result<OAuthTokens>;

    /// Get user information using access token
    async fn get_user(&self, token: &str) -> crate::Result<OAuthUser>;

    /// Refresh access token using refresh token (if supported)
    async fn refresh_token(&self, refresh_token: &str) -> crate::Result<OAuthTokens> {
        Err(crate::OAuthError::RefreshError(
            "Token refresh not supported by this provider".to_string(),
        ))
    }

    /// Revoke access token (if supported)
    async fn revoke_token(&self, _token: &str) -> crate::Result<()> {
        Ok(())
    }
}
