//! OAuth2 driver implementation

use crate::providers::Provider;
use crate::user::{User, UserData};
use crate::pkce::Pkce;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum SocialiteError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SocialiteError>;

/// OAuth2 token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// OAuth2 driver
pub struct Driver {
    provider: Provider,
    client_id: String,
    client_secret: String,
    redirect_url: String,
    scopes: Vec<String>,
    state: Option<String>,
    pkce: Option<Pkce>,
    use_pkce: bool,
}

impl Driver {
    /// Create a new driver for the given provider
    pub fn new(provider: Provider) -> DriverBuilder {
        DriverBuilder::new(provider)
    }

    /// Get the authorization URL to redirect users to
    pub fn redirect(&mut self) -> Result<String> {
        let mut url = Url::parse(&self.provider.authorize_url())?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_url)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.scopes.join(" "));

        if let Some(state) = &self.state {
            url.query_pairs_mut().append_pair("state", state);
        }

        // Add PKCE parameters if enabled
        if self.use_pkce {
            let pkce = Pkce::generate();
            url.query_pairs_mut()
                .append_pair("code_challenge", &pkce.code_challenge)
                .append_pair("code_challenge_method", &pkce.code_challenge_method);
            self.pkce = Some(pkce);
        }

        Ok(url.to_string())
    }

    /// Exchange authorization code for access token
    pub async fn get_access_token(&self, code: &str) -> Result<TokenResponse> {
        let client = reqwest::Client::new();

        let mut params = vec![
            ("grant_type", "authorization_code".to_string()),
            ("client_id", self.client_id.clone()),
            ("client_secret", self.client_secret.clone()),
            ("code", code.to_string()),
            ("redirect_uri", self.redirect_url.clone()),
        ];

        // Add PKCE code verifier if used
        if let Some(pkce) = &self.pkce {
            params.push(("code_verifier", pkce.code_verifier.clone()));
        }

        let response = client
            .post(&self.provider.token_url())
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(SocialiteError::OAuthError(error));
        }

        let token = response.json::<TokenResponse>().await?;
        Ok(token)
    }

    /// Get user information from the provider
    pub async fn user_from_code(&self, code: &str) -> Result<User> {
        let token = self.get_access_token(code).await?;
        self.user_from_token(&token.access_token).await
    }

    /// Get user information using an access token
    pub async fn user_from_token(&self, access_token: &str) -> Result<User> {
        let client = reqwest::Client::new();

        let response = client
            .get(&self.provider.user_url())
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(SocialiteError::OAuthError(error));
        }

        let user_data: UserData = response.json().await?;

        Ok(User {
            id: user_data.id.clone().unwrap_or_default(),
            name: user_data.name.clone().unwrap_or_default(),
            email: user_data.email.clone(),
            avatar: user_data.avatar_url.clone(),
            provider: self.provider.name().to_string(),
            token: access_token.to_string(),
            raw: user_data,
        })
    }

    /// Refresh an access token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let client = reqwest::Client::new();

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("refresh_token", refresh_token),
        ];

        let response = client
            .post(&self.provider.token_url())
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(SocialiteError::OAuthError(error));
        }

        let token = response.json::<TokenResponse>().await?;
        Ok(token)
    }
}

/// Builder for OAuth2 driver
pub struct DriverBuilder {
    provider: Provider,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_url: Option<String>,
    scopes: Vec<String>,
    state: Option<String>,
    use_pkce: bool,
}

impl DriverBuilder {
    pub fn new(provider: Provider) -> Self {
        Self {
            scopes: provider.default_scopes(),
            provider,
            client_id: None,
            client_secret: None,
            redirect_url: None,
            state: None,
            use_pkce: false,
        }
    }

    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    pub fn client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = Some(client_secret.into());
        self
    }

    pub fn redirect_url(mut self, redirect_url: impl Into<String>) -> Self {
        self.redirect_url = Some(redirect_url.into());
        self
    }

    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    pub fn state(mut self, state: impl Into<String>) -> Self {
        self.state = Some(state.into());
        self
    }

    /// Enable PKCE (Proof Key for Code Exchange)
    pub fn with_pkce(mut self) -> Self {
        self.use_pkce = true;
        self
    }

    pub fn build(self) -> Result<Driver> {
        let client_id = self.client_id
            .ok_or_else(|| SocialiteError::InvalidConfig("client_id is required".to_string()))?;
        let client_secret = self.client_secret
            .ok_or_else(|| SocialiteError::InvalidConfig("client_secret is required".to_string()))?;
        let redirect_url = self.redirect_url
            .ok_or_else(|| SocialiteError::InvalidConfig("redirect_url is required".to_string()))?;

        Ok(Driver {
            provider: self.provider,
            client_id,
            client_secret,
            redirect_url,
            scopes: self.scopes,
            state: self.state,
            pkce: None,
            use_pkce: self.use_pkce,
        })
    }
}

/// Main Socialite facade
pub struct Socialite;

impl Socialite {
    /// Create a driver for the given provider
    pub fn driver(provider: Provider) -> DriverBuilder {
        Driver::new(provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_builder() {
        let driver = Socialite::driver(Provider::GitHub)
            .client_id("test-id")
            .client_secret("test-secret")
            .redirect_url("http://localhost/callback")
            .build()
            .unwrap();

        assert_eq!(driver.client_id, "test-id");
        assert_eq!(driver.client_secret, "test-secret");
        assert_eq!(driver.redirect_url, "http://localhost/callback");
    }

    #[test]
    fn test_driver_builder_missing_config() {
        let result = Socialite::driver(Provider::GitHub)
            .client_id("test-id")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_redirect_url() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("test-id")
            .client_secret("test-secret")
            .redirect_url("http://localhost/callback")
            .build()
            .unwrap();

        let url = driver.redirect().unwrap();
        assert!(url.contains("client_id=test-id"));
        assert!(url.contains("redirect_uri=http"));
    }

    #[test]
    fn test_custom_scopes() {
        let mut driver = Socialite::driver(Provider::GitHub)
            .client_id("test-id")
            .client_secret("test-secret")
            .redirect_url("http://localhost/callback")
            .scope("user:email")
            .scope("repo")
            .build()
            .unwrap();

        let url = driver.redirect().unwrap();
        assert!(url.contains("scope="));
    }
}
