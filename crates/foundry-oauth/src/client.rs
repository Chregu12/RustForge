//! OAuth client

use crate::{OAuthProvider, OAuthUser, OAuthTokens, StateManager, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// OAuth client for managing multiple providers and authentication flows
pub struct OAuthClient {
    providers: HashMap<String, Box<dyn OAuthProvider>>,
    state_manager: Arc<StateManager>,
}

impl OAuthClient {
    /// Create a new OAuth client
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            state_manager: Arc::new(StateManager::default()),
        }
    }

    /// Create a new OAuth client with custom state manager
    pub fn with_state_manager(state_manager: StateManager) -> Self {
        Self {
            providers: HashMap::new(),
            state_manager: Arc::new(state_manager),
        }
    }

    /// Register an OAuth provider
    pub fn register_provider(&mut self, provider: Box<dyn OAuthProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get a provider by name
    pub fn get_provider(&self, name: &str) -> Option<&dyn OAuthProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Get the authorization URL with a generated state parameter
    pub async fn get_authorize_url(&self, provider_name: &str) -> Result<(String, String)> {
        let provider = self
            .get_provider(provider_name)
            .ok_or_else(|| crate::OAuthError::ProviderError("Provider not found".to_string()))?;

        let state = self.state_manager.generate().await;
        let url = provider.authorize_url(&state);

        Ok((url, state))
    }

    /// Authenticate a user with authorization code and state validation
    pub async fn authenticate(
        &self,
        provider_name: &str,
        code: &str,
        state: &str,
    ) -> Result<(OAuthUser, OAuthTokens)> {
        // Validate state parameter for CSRF protection
        if !self.state_manager.validate(state).await {
            return Err(crate::OAuthError::InvalidState);
        }

        let provider = self
            .get_provider(provider_name)
            .ok_or_else(|| crate::OAuthError::ProviderError("Provider not found".to_string()))?;

        let tokens = provider.exchange_code(code).await?;
        let user = provider.get_user(&tokens.access_token).await?;

        Ok((user, tokens))
    }

    /// Refresh an access token
    pub async fn refresh_token(
        &self,
        provider_name: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokens> {
        let provider = self
            .get_provider(provider_name)
            .ok_or_else(|| crate::OAuthError::ProviderError("Provider not found".to_string()))?;

        provider.refresh_token(refresh_token).await
    }

    /// Revoke an access token
    pub async fn revoke_token(&self, provider_name: &str, token: &str) -> Result<()> {
        let provider = self
            .get_provider(provider_name)
            .ok_or_else(|| crate::OAuthError::ProviderError("Provider not found".to_string()))?;

        provider.revoke_token(token).await
    }
}

impl Default for OAuthClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::OAuthProvider;
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl OAuthProvider for MockProvider {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn authorize_url(&self, state: &str) -> String {
            format!("https://mock.example.com/auth?state={}", state)
        }

        async fn exchange_code(&self, _code: &str) -> Result<OAuthTokens> {
            Ok(OAuthTokens {
                access_token: "mock_access_token".to_string(),
                refresh_token: Some("mock_refresh_token".to_string()),
                expires_in: Some(3600),
                token_type: "Bearer".to_string(),
            })
        }

        async fn get_user(&self, _token: &str) -> Result<OAuthUser> {
            Ok(OAuthUser {
                provider: "mock".to_string(),
                provider_id: "123".to_string(),
                email: Some("test@example.com".to_string()),
                name: Some("Test User".to_string()),
                avatar: None,
            })
        }
    }

    #[tokio::test]
    async fn test_register_provider() {
        let mut client = OAuthClient::new();
        client.register_provider(Box::new(MockProvider));

        assert!(client.get_provider("mock").is_some());
        assert!(client.get_provider("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_get_authorize_url() {
        let mut client = OAuthClient::new();
        client.register_provider(Box::new(MockProvider));

        let (url, state) = client.get_authorize_url("mock").await.unwrap();

        assert!(url.contains("https://mock.example.com/auth"));
        assert!(url.contains(&state));
        assert_eq!(state.len(), 32);
    }

    #[tokio::test]
    async fn test_authenticate_with_valid_state() {
        let mut client = OAuthClient::new();
        client.register_provider(Box::new(MockProvider));

        let (_url, state) = client.get_authorize_url("mock").await.unwrap();
        let (user, tokens) = client
            .authenticate("mock", "code123", &state)
            .await
            .unwrap();

        assert_eq!(user.email, Some("test@example.com".to_string()));
        assert_eq!(tokens.access_token, "mock_access_token");
    }

    #[tokio::test]
    async fn test_authenticate_with_invalid_state() {
        let mut client = OAuthClient::new();
        client.register_provider(Box::new(MockProvider));

        let result = client.authenticate("mock", "code123", "invalid_state").await;

        assert!(matches!(result, Err(crate::OAuthError::InvalidState)));
    }
}

