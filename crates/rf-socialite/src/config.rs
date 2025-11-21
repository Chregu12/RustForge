//! Configuration for OAuth2 providers
//!
//! This module provides configuration structures for OAuth2 providers,
//! typically loaded from environment variables.

use serde::{Deserialize, Serialize};
use std::env;

/// Configuration for a single OAuth2 provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// OAuth2 client ID
    pub client_id: String,

    /// OAuth2 client secret
    pub client_secret: String,

    /// Redirect URI for OAuth2 callback
    pub redirect_uri: String,

    /// OAuth2 scopes
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl ProviderConfig {
    /// Create a new provider configuration
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            redirect_uri: redirect_uri.into(),
            scopes: Vec::new(),
        }
    }

    /// Set scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Load from environment variables with prefix
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_socialite::config::ProviderConfig;
    ///
    /// // Loads from GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, GOOGLE_REDIRECT_URI
    /// let config = ProviderConfig::from_env("GOOGLE");
    /// ```
    pub fn from_env(prefix: &str) -> Result<Self, ConfigError> {
        let client_id = env::var(format!("{}_CLIENT_ID", prefix))
            .map_err(|_| ConfigError::MissingEnvVar(format!("{}_CLIENT_ID", prefix)))?;
        let client_secret = env::var(format!("{}_CLIENT_SECRET", prefix))
            .map_err(|_| ConfigError::MissingEnvVar(format!("{}_CLIENT_SECRET", prefix)))?;
        let redirect_uri = env::var(format!("{}_REDIRECT_URI", prefix))
            .map_err(|_| ConfigError::MissingEnvVar(format!("{}_REDIRECT_URI", prefix)))?;

        let scopes = env::var(format!("{}_SCOPES", prefix))
            .ok()
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            scopes,
        })
    }
}

/// Complete Socialite configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocialiteConfig {
    /// Google OAuth configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google: Option<ProviderConfig>,

    /// GitHub OAuth configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<ProviderConfig>,

    /// Facebook OAuth configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<ProviderConfig>,

    /// Twitter OAuth configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter: Option<ProviderConfig>,
}

impl SocialiteConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_socialite::config::SocialiteConfig;
    ///
    /// let config = SocialiteConfig::from_env();
    /// ```
    pub fn from_env() -> Self {
        Self {
            google: ProviderConfig::from_env("GOOGLE").ok(),
            github: ProviderConfig::from_env("GITHUB").ok(),
            facebook: ProviderConfig::from_env("FACEBOOK").ok(),
            twitter: ProviderConfig::from_env("TWITTER").ok(),
        }
    }

    /// Set Google configuration
    pub fn with_google(mut self, config: ProviderConfig) -> Self {
        self.google = Some(config);
        self
    }

    /// Set GitHub configuration
    pub fn with_github(mut self, config: ProviderConfig) -> Self {
        self.github = Some(config);
        self
    }

    /// Set Facebook configuration
    pub fn with_facebook(mut self, config: ProviderConfig) -> Self {
        self.facebook = Some(config);
        self
    }

    /// Set Twitter configuration
    pub fn with_twitter(mut self, config: ProviderConfig) -> Self {
        self.twitter = Some(config);
        self
    }

    /// Get provider configuration by name
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        match name.to_lowercase().as_str() {
            "google" => self.google.as_ref(),
            "github" => self.github.as_ref(),
            "facebook" => self.facebook.as_ref(),
            "twitter" => self.twitter.as_ref(),
            _ => None,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_new() {
        let config = ProviderConfig::new("client-id", "client-secret", "http://localhost/callback");
        assert_eq!(config.client_id, "client-id");
        assert_eq!(config.client_secret, "client-secret");
        assert_eq!(config.redirect_uri, "http://localhost/callback");
    }

    #[test]
    fn test_provider_config_with_scopes() {
        let config = ProviderConfig::new("id", "secret", "uri")
            .with_scopes(vec!["email".to_string(), "profile".to_string()]);
        assert_eq!(config.scopes.len(), 2);
    }

    #[test]
    fn test_socialite_config_builder() {
        let google_config = ProviderConfig::new("google-id", "google-secret", "google-uri");
        let github_config = ProviderConfig::new("github-id", "github-secret", "github-uri");

        let config = SocialiteConfig::new()
            .with_google(google_config)
            .with_github(github_config);

        assert!(config.google.is_some());
        assert!(config.github.is_some());
        assert!(config.facebook.is_none());
    }

    #[test]
    fn test_get_provider() {
        let google_config = ProviderConfig::new("google-id", "google-secret", "google-uri");
        let config = SocialiteConfig::new().with_google(google_config);

        assert!(config.get_provider("google").is_some());
        assert!(config.get_provider("github").is_none());
        assert!(config.get_provider("GOOGLE").is_some()); // Case insensitive
    }
}
