//! OAuth2 Server
//!
//! Main OAuth2 server orchestrating all grants and services

use crate::clients::ClientRepository;
use crate::errors::{OAuth2Error, OAuth2Result};
use crate::grants::{
    AuthCodeParams, AuthorizationCodeGrant, ClientCredentialsGrant, PasswordGrant, RefreshTokenGrant,
    TokenResponse,
};
use crate::models::{AccessToken, AuthorizationCode, Client, PersonalAccessToken, RefreshToken};
use crate::scopes::ScopeManager;
use crate::tokens::{TokenGenerator, TokenValidator};
use crate::OAuth2Config;
use std::sync::Arc;
use uuid::Uuid;

/// OAuth2 Server
pub struct OAuth2Server<R: ClientRepository> {
    config: OAuth2Config,
    client_repository: Arc<R>,
    scope_manager: Arc<ScopeManager>,
    #[allow(dead_code)]
    token_generator: TokenGenerator,
    token_validator: TokenValidator,
    authorization_code_grant: AuthorizationCodeGrant,
    client_credentials_grant: ClientCredentialsGrant,
    password_grant: PasswordGrant,
    refresh_token_grant: RefreshTokenGrant,
}

impl<R: ClientRepository> OAuth2Server<R> {
    pub fn new(config: OAuth2Config, client_repository: R) -> Self {
        let token_generator = TokenGenerator::new(config.jwt_secret.clone(), config.issuer.clone());
        let token_validator = TokenValidator::new(config.jwt_secret.clone(), config.issuer.clone());

        let authorization_code_grant = AuthorizationCodeGrant::new(
            token_generator.clone(),
            config.access_token_lifetime,
            config.refresh_token_lifetime,
        );

        let client_credentials_grant =
            ClientCredentialsGrant::new(token_generator.clone(), config.access_token_lifetime);

        let password_grant = PasswordGrant::new(
            token_generator.clone(),
            config.access_token_lifetime,
            config.refresh_token_lifetime,
        );

        let refresh_token_grant = RefreshTokenGrant::new(
            token_generator.clone(),
            config.access_token_lifetime,
            config.refresh_token_lifetime,
        );

        Self {
            config,
            client_repository: Arc::new(client_repository),
            scope_manager: Arc::new(ScopeManager::with_defaults()),
            token_generator,
            token_validator,
            authorization_code_grant,
            client_credentials_grant,
            password_grant,
            refresh_token_grant,
        }
    }

    /// Get client repository
    pub fn client_repository(&self) -> Arc<R> {
        Arc::clone(&self.client_repository)
    }

    /// Get scope manager
    pub fn scope_manager(&self) -> Arc<ScopeManager> {
        Arc::clone(&self.scope_manager)
    }

    /// Validate client credentials
    pub async fn validate_client(
        &self,
        client_id: Uuid,
        client_secret: Option<&str>,
    ) -> OAuth2Result<Client> {
        let client = if let Some(secret) = client_secret {
            // Confidential client authentication
            self.client_repository
                .find_by_credentials(client_id, secret)
                .await?
                .ok_or_else(|| OAuth2Error::InvalidClient("Invalid client credentials".to_string()))?
        } else {
            // Public client
            self.client_repository
                .find(client_id)
                .await?
                .ok_or_else(|| OAuth2Error::InvalidClient("Client not found".to_string()))?
        };

        // Check if client is revoked
        if client.revoked {
            return Err(OAuth2Error::InvalidClient("Client revoked".to_string()));
        }

        Ok(client)
    }

    /// Validate scopes against client and scope manager
    pub fn validate_scopes(&self, client: &Client, requested_scopes: &[String]) -> OAuth2Result<Vec<String>> {
        // Validate scopes exist
        let validated = self
            .scope_manager
            .validate(requested_scopes)
            .map_err(OAuth2Error::InvalidScope)?;

        // Check client is allowed to request these scopes
        for scope in &validated {
            if !client.has_scope(scope) {
                return Err(OAuth2Error::InvalidScope(format!(
                    "Client not authorized for scope: {}",
                    scope
                )));
            }
        }

        Ok(validated)
    }

    /// Validate redirect URI
    pub fn validate_redirect_uri(&self, client: &Client, redirect_uri: &str) -> OAuth2Result<()> {
        if !client.redirect_uris.contains(&redirect_uri.to_string()) {
            return Err(OAuth2Error::InvalidRequest(
                "Invalid redirect_uri".to_string(),
            ));
        }
        Ok(())
    }

    /// Authorization Code Flow: Create authorization code
    pub fn create_authorization_code(
        &self,
        client: &Client,
        user_id: Uuid,
        redirect_uri: String,
        scopes: Vec<String>,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> OAuth2Result<AuthorizationCode> {
        // Validate grant type
        if !client.supports_grant("authorization_code") {
            return Err(OAuth2Error::UnauthorizedClient(
                "Client not authorized for authorization_code grant".to_string(),
            ));
        }

        // Validate redirect URI
        self.validate_redirect_uri(client, &redirect_uri)?;

        // Validate scopes
        let validated_scopes = self.validate_scopes(client, &scopes)?;

        // PKCE required for public clients
        if !client.is_confidential() && code_challenge.is_none() {
            return Err(OAuth2Error::InvalidRequest(
                "PKCE required for public clients".to_string(),
            ));
        }

        let params = AuthCodeParams {
            user_id,
            redirect_uri,
            scopes: validated_scopes,
            code_challenge,
            code_challenge_method,
            lifetime: self.config.auth_code_lifetime,
        };

        self.authorization_code_grant.create_authorization_code(client, params)
    }

    /// Authorization Code Flow: Exchange code for tokens
    pub async fn exchange_authorization_code(
        &self,
        client: &Client,
        code: &AuthorizationCode,
        code_verifier: Option<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Validate client owns this code
        if code.client_id != client.id {
            return Err(OAuth2Error::InvalidGrant(
                "Code issued to different client".to_string(),
            ));
        }

        self.authorization_code_grant
            .exchange_code(client, code, code_verifier)
            .await
    }

    /// Client Credentials Flow
    pub async fn issue_client_credentials_token(
        &self,
        client: &Client,
        scopes: Vec<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Validate grant type
        if !client.supports_grant("client_credentials") {
            return Err(OAuth2Error::UnauthorizedClient(
                "Client not authorized for client_credentials grant".to_string(),
            ));
        }

        // Validate scopes
        let validated_scopes = self.validate_scopes(client, &scopes)?;

        self.client_credentials_grant
            .issue_token(client, validated_scopes)
            .await
    }

    /// Password Flow
    pub async fn issue_password_token(
        &self,
        client: &Client,
        user_id: Uuid,
        scopes: Vec<String>,
    ) -> OAuth2Result<TokenResponse> {
        // Validate grant type
        if !client.supports_grant("password") {
            return Err(OAuth2Error::UnauthorizedClient(
                "Client not authorized for password grant".to_string(),
            ));
        }

        // Validate scopes
        let validated_scopes = self.validate_scopes(client, &scopes)?;

        self.password_grant
            .issue_token(client, user_id, validated_scopes)
            .await
    }

    /// Refresh Token Flow
    pub async fn refresh_token(
        &self,
        client: &Client,
        refresh_token: &RefreshToken,
        access_token: &AccessToken,
    ) -> OAuth2Result<TokenResponse> {
        // Validate grant type
        if !client.supports_grant("refresh_token") {
            return Err(OAuth2Error::UnauthorizedClient(
                "Client not authorized for refresh_token grant".to_string(),
            ));
        }

        // Validate refresh token belongs to this access token
        if refresh_token.access_token_id != access_token.id {
            return Err(OAuth2Error::InvalidGrant(
                "Refresh token does not match access token".to_string(),
            ));
        }

        // Validate access token belongs to this client
        if access_token.client_id != client.id {
            return Err(OAuth2Error::InvalidGrant(
                "Token issued to different client".to_string(),
            ));
        }

        self.refresh_token_grant
            .refresh(client, refresh_token, access_token)
            .await
    }

    /// Create Personal Access Token
    pub fn create_personal_access_token(
        &self,
        user_id: Uuid,
        name: String,
        scopes: Vec<String>,
    ) -> OAuth2Result<PersonalAccessToken> {
        // Validate scopes
        self.scope_manager
            .validate(&scopes)
            .map_err(OAuth2Error::InvalidScope)?;

        Ok(PersonalAccessToken::new(user_id, name, scopes))
    }

    /// Validate access token
    pub fn validate_token(&self, token: &str) -> OAuth2Result<crate::tokens::TokenClaims> {
        self.token_validator.validate_access_token(token)
    }

    /// Introspect token
    pub fn introspect_token(&self, token: &str) -> crate::tokens::TokenIntrospection {
        self.token_validator.introspect(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::InMemoryClientRepository;

    #[tokio::test]
    async fn test_oauth2_server_creation() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();
        let server = OAuth2Server::new(config, repo);

        assert!(server.scope_manager().exists("*"));
        assert!(server.scope_manager().exists("users:read"));
    }

    #[tokio::test]
    async fn test_validate_client() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;
        let secret = client.secret.clone().unwrap();

        repo.store(client).await.unwrap();

        let server = OAuth2Server::new(config, repo);

        // Valid client
        let validated = server
            .validate_client(client_id, Some(&secret))
            .await
            .unwrap();
        assert_eq!(validated.name, "Test App");

        // Invalid secret
        let result = server.validate_client(client_id, Some("wrong")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authorization_code_flow() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();

        let client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        let client_id = client.id;

        repo.store(client.clone()).await.unwrap();

        let server = OAuth2Server::new(config, repo);

        let user_id = Uuid::new_v4();
        let scopes = vec!["users:read".to_string()];

        // Create authorization code
        let code = server
            .create_authorization_code(
                &client,
                user_id,
                "http://localhost/callback".to_string(),
                scopes,
                None,
                None,
            )
            .unwrap();

        assert_eq!(code.client_id, client_id);
        assert_eq!(code.user_id, user_id);

        // Exchange code for tokens
        let token_response = server
            .exchange_authorization_code(&client, &code, None)
            .await
            .unwrap();

        assert!(!token_response.access_token.is_empty());
        assert!(token_response.refresh_token.is_some());
    }

    #[tokio::test]
    async fn test_client_credentials_flow() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();

        let mut client = Client::new(
            "Test App".to_string(),
            vec!["http://localhost/callback".to_string()],
        );
        client.grants.push("client_credentials".to_string());

        repo.store(client.clone()).await.unwrap();

        let server = OAuth2Server::new(config, repo);

        let scopes = vec!["api:read".to_string()];

        let token_response = server
            .issue_client_credentials_token(&client, scopes)
            .await
            .unwrap();

        assert!(!token_response.access_token.is_empty());
        assert!(token_response.refresh_token.is_none()); // No refresh token for client credentials
    }

    #[tokio::test]
    async fn test_personal_access_token() {
        let config = OAuth2Config::default();
        let repo = InMemoryClientRepository::new();
        let server = OAuth2Server::new(config, repo);

        let user_id = Uuid::new_v4();
        let token = server
            .create_personal_access_token(
                user_id,
                "My API Token".to_string(),
                vec!["users:read".to_string()],
            )
            .unwrap();

        assert_eq!(token.user_id, user_id);
        assert_eq!(token.name, "My API Token");
    }
}
