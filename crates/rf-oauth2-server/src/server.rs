//! OAuth2 server implementation

use crate::{
    client::Client, error::{OAuth2Error, OAuth2Result}, token::{AccessToken, RefreshToken, TokenResponse}, types::{GrantType, OAuth2Config, Scope}
};
use chrono::{Duration, Utc};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// OAuth2 authorization server
#[derive(Clone)]
pub struct OAuth2Server {
    config: OAuth2Config,
    clients: Arc<RwLock<HashMap<String, Client>>>,
    access_tokens: Arc<RwLock<HashMap<String, AccessToken>>>,
    refresh_tokens: Arc<RwLock<HashMap<String, RefreshToken>>>,
    auth_codes: Arc<RwLock<HashMap<String, AuthorizationCode>>>,
}

#[derive(Clone)]
struct AuthorizationCode {
    code: String,
    client_id: String,
    redirect_uri: String,
    scopes: Vec<Scope>,
    user_id: Option<String>,
    expires_at: chrono::DateTime<Utc>,
    code_challenge: Option<String>,
}

impl OAuth2Server {
    /// Create new OAuth2 server
    pub fn new(config: OAuth2Config) -> Self {
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            access_tokens: Arc::new(RwLock::new(HashMap::new())),
            refresh_tokens: Arc::new(RwLock::new(HashMap::new())),
            auth_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a client
    pub async fn register_client(&self, client: Client) -> OAuth2Result<()> {
        let mut clients = self.clients.write().await;
        clients.insert(client.id.clone(), client);
        Ok(())
    }

    /// Get client by ID
    pub async fn get_client(&self, client_id: &str) -> OAuth2Result<Client> {
        let clients = self.clients.read().await;
        clients
            .get(client_id)
            .cloned()
            .ok_or_else(|| OAuth2Error::InvalidClient("Client not found".to_string()))
    }

    /// Generate authorization code
    pub async fn generate_authorization_code(
        &self,
        client_id: String,
        redirect_uri: String,
        scopes: Vec<Scope>,
        user_id: Option<String>,
        code_challenge: Option<String>,
    ) -> OAuth2Result<String> {
        let code = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::minutes(10);

        let auth_code = AuthorizationCode {
            code: code.clone(),
            client_id,
            redirect_uri,
            scopes,
            user_id,
            expires_at,
            code_challenge,
        };

        let mut codes = self.auth_codes.write().await;
        codes.insert(code.clone(), auth_code);

        Ok(code)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(
        &self,
        code: &str,
        client_id: &str,
        client_secret: Option<&str>,
        redirect_uri: &str,
        code_verifier: Option<&str>,
    ) -> OAuth2Result<TokenResponse> {
        // Get and remove authorization code
        let auth_code = {
            let mut codes = self.auth_codes.write().await;
            codes
                .remove(code)
                .ok_or_else(|| OAuth2Error::InvalidGrant("Invalid authorization code".to_string()))?
        };

        // Verify code not expired
        if Utc::now() > auth_code.expires_at {
            return Err(OAuth2Error::InvalidGrant("Code expired".to_string()));
        }

        // Verify client
        if auth_code.client_id != client_id {
            return Err(OAuth2Error::InvalidClient("Client mismatch".to_string()));
        }

        // Verify redirect URI
        if auth_code.redirect_uri != redirect_uri {
            return Err(OAuth2Error::InvalidGrant(
                "Redirect URI mismatch".to_string(),
            ));
        }

        // Verify PKCE if present
        if let Some(challenge) = auth_code.code_challenge {
            let verifier = code_verifier.ok_or_else(|| {
                OAuth2Error::InvalidRequest("Code verifier required".to_string())
            })?;

            // Simple verification (in production, use proper PKCE)
            if challenge != verifier {
                return Err(OAuth2Error::InvalidGrant(
                    "Invalid code verifier".to_string(),
                ));
            }
        }

        // Verify client credentials
        let client = self.get_client(client_id).await?;
        if let Some(secret) = client_secret {
            if !client.verify_secret(secret) {
                return Err(OAuth2Error::UnauthorizedClient);
            }
        }

        // Generate tokens
        self.generate_tokens(client_id, auth_code.user_id, auth_code.scopes)
            .await
    }

    /// Client credentials flow
    pub async fn client_credentials(
        &self,
        client_id: &str,
        client_secret: &str,
        scopes: Vec<Scope>,
    ) -> OAuth2Result<TokenResponse> {
        // Verify client
        let client = self.get_client(client_id).await?;

        if !client.supports_grant(&GrantType::ClientCredentials) {
            return Err(OAuth2Error::UnsupportedGrantType(
                "Client credentials not supported".to_string(),
            ));
        }

        if !client.verify_secret(client_secret) {
            return Err(OAuth2Error::UnauthorizedClient);
        }

        // Verify scopes
        for scope in &scopes {
            if !client.is_scope_valid(scope) {
                return Err(OAuth2Error::InvalidScope(format!(
                    "Invalid scope: {}",
                    scope
                )));
            }
        }

        // Generate access token (no refresh token for client credentials)
        let token_id = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::seconds(self.config.access_token_ttl as i64);

        let access_token = AccessToken {
            token: token_id.clone(),
            expires_at,
            client_id: client_id.to_string(),
            user_id: None,
            scopes: scopes.clone(),
        };

        let mut tokens = self.access_tokens.write().await;
        tokens.insert(token_id.clone(), access_token);

        Ok(TokenResponse::bearer(token_id, self.config.access_token_ttl).with_scopes(scopes))
    }

    /// Validate access token
    pub async fn validate_token(&self, token: &str) -> OAuth2Result<AccessToken> {
        let tokens = self.access_tokens.read().await;
        let access_token = tokens
            .get(token)
            .ok_or_else(|| OAuth2Error::InvalidToken("Token not found".to_string()))?;

        if access_token.is_expired() {
            return Err(OAuth2Error::InvalidToken("Token expired".to_string()));
        }

        Ok(access_token.clone())
    }

    /// Revoke token
    pub async fn revoke_token(&self, token: &str) -> OAuth2Result<()> {
        let mut tokens = self.access_tokens.write().await;
        tokens.remove(token);
        Ok(())
    }

    /// Generate access and refresh tokens
    async fn generate_tokens(
        &self,
        client_id: &str,
        user_id: Option<String>,
        scopes: Vec<Scope>,
    ) -> OAuth2Result<TokenResponse> {
        let access_token_id = Uuid::new_v4().to_string();
        let refresh_token_id = Uuid::new_v4().to_string();

        let access_expires = Utc::now() + Duration::seconds(self.config.access_token_ttl as i64);
        let refresh_expires = Utc::now() + Duration::seconds(self.config.refresh_token_ttl as i64);

        let access_token = AccessToken {
            token: access_token_id.clone(),
            expires_at: access_expires,
            client_id: client_id.to_string(),
            user_id: user_id.clone(),
            scopes: scopes.clone(),
        };

        let refresh_token = RefreshToken {
            token: refresh_token_id.clone(),
            expires_at: refresh_expires,
            client_id: client_id.to_string(),
            user_id,
            scopes: scopes.clone(),
        };

        let mut access_tokens = self.access_tokens.write().await;
        access_tokens.insert(access_token_id.clone(), access_token);

        let mut refresh_tokens = self.refresh_tokens.write().await;
        refresh_tokens.insert(refresh_token_id.clone(), refresh_token);

        Ok(TokenResponse::bearer(
            access_token_id,
            self.config.access_token_ttl,
        )
        .with_refresh_token(refresh_token_id)
        .with_scopes(scopes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get_client() {
        let oauth = OAuth2Server::new(OAuth2Config::default());

        let client = Client {
            id: "test-client".to_string(),
            secret: Some("secret".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            grants: vec![GrantType::ClientCredentials],
            scopes: vec!["read".to_string()],
        };

        oauth.register_client(client.clone()).await.unwrap();

        let fetched = oauth.get_client("test-client").await.unwrap();
        assert_eq!(fetched.id, "test-client");
    }

    #[tokio::test]
    async fn test_client_credentials_flow() {
        let oauth = OAuth2Server::new(OAuth2Config::default());

        let client = Client {
            id: "test-client".to_string(),
            secret: Some("secret123".to_string()),
            redirect_uris: vec![],
            grants: vec![GrantType::ClientCredentials],
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        oauth.register_client(client).await.unwrap();

        let response = oauth
            .client_credentials(
                "test-client",
                "secret123",
                vec!["read".to_string()],
            )
            .await
            .unwrap();

        assert!(!response.access_token.is_empty());
        assert_eq!(response.token_type, "Bearer");
    }

    #[tokio::test]
    async fn test_token_validation() {
        let oauth = OAuth2Server::new(OAuth2Config::default());

        let client = Client {
            id: "test-client".to_string(),
            secret: Some("secret".to_string()),
            redirect_uris: vec![],
            grants: vec![GrantType::ClientCredentials],
            scopes: vec!["read".to_string()],
        };

        oauth.register_client(client).await.unwrap();

        let response = oauth
            .client_credentials("test-client", "secret", vec!["read".to_string()])
            .await
            .unwrap();

        let validated = oauth.validate_token(&response.access_token).await.unwrap();
        assert_eq!(validated.client_id, "test-client");
        assert!(validated.has_scope("read"));
    }

    #[tokio::test]
    async fn test_authorization_code_flow() {
        let oauth = OAuth2Server::new(OAuth2Config::default());

        let client = Client {
            id: "test-client".to_string(),
            secret: Some("secret".to_string()),
            redirect_uris: vec!["https://example.com/callback".to_string()],
            grants: vec![GrantType::AuthorizationCode],
            scopes: vec!["read".to_string()],
        };

        oauth.register_client(client).await.unwrap();

        // Generate authorization code
        let code = oauth
            .generate_authorization_code(
                "test-client".to_string(),
                "https://example.com/callback".to_string(),
                vec!["read".to_string()],
                Some("user-123".to_string()),
                None,
            )
            .await
            .unwrap();

        // Exchange code for tokens
        let response = oauth
            .exchange_code(
                &code,
                "test-client",
                Some("secret"),
                "https://example.com/callback",
                None,
            )
            .await
            .unwrap();

        assert!(!response.access_token.is_empty());
        assert!(response.refresh_token.is_some());
    }
}
