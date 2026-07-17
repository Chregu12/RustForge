//! Configuration for OAuth2 Passport

use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Passport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportConfig {
    /// Access token lifetime (in seconds)
    pub access_token_lifetime: i64,

    /// Refresh token lifetime (in seconds)
    pub refresh_token_lifetime: i64,

    /// Authorization code lifetime (in seconds)
    pub auth_code_lifetime: i64,

    /// Personal access token lifetime (in seconds, None = never expires)
    pub personal_access_token_lifetime: Option<i64>,

    /// Enforce PKCE for authorization code flow
    pub enforce_pkce: bool,

    /// Allow plain text PKCE challenge method (not recommended for production)
    pub allow_plain_pkce: bool,

    /// Default scopes if none specified
    pub default_scopes: Vec<String>,

    /// Enable password grant (not recommended for production)
    pub enable_password_grant: bool,

    /// Enable implicit grant (deprecated, not recommended)
    pub enable_implicit_grant: bool,

    /// Enable client credentials grant
    pub enable_client_credentials_grant: bool,

    /// Enable authorization code grant
    pub enable_authorization_code_grant: bool,

    /// Enable refresh token grant
    pub enable_refresh_token_grant: bool,

    /// Require client authentication for token endpoint
    pub require_client_authentication: bool,

    /// Token length (in characters)
    pub token_length: usize,
}

impl Default for PassportConfig {
    fn default() -> Self {
        Self {
            // 1 hour
            access_token_lifetime: 3600,
            // 30 days
            refresh_token_lifetime: 2592000,
            // 10 minutes
            auth_code_lifetime: 600,
            // Never expires by default
            personal_access_token_lifetime: None,
            // Enforce PKCE by default
            enforce_pkce: true,
            // Disallow plain text PKCE
            allow_plain_pkce: false,
            // No default scopes
            default_scopes: vec![],
            // Disable password grant by default (security best practice)
            enable_password_grant: false,
            // Disable implicit grant by default (deprecated)
            enable_implicit_grant: false,
            // Enable client credentials grant
            enable_client_credentials_grant: true,
            // Enable authorization code grant
            enable_authorization_code_grant: true,
            // Enable refresh token grant
            enable_refresh_token_grant: true,
            // Require client authentication
            require_client_authentication: true,
            // 80 character tokens
            token_length: 80,
        }
    }
}

impl PassportConfig {
    /// Create a new config with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set access token lifetime
    pub fn access_token_lifetime(mut self, seconds: i64) -> Self {
        self.access_token_lifetime = seconds;
        self
    }

    /// Set refresh token lifetime
    pub fn refresh_token_lifetime(mut self, seconds: i64) -> Self {
        self.refresh_token_lifetime = seconds;
        self
    }

    /// Set authorization code lifetime
    pub fn auth_code_lifetime(mut self, seconds: i64) -> Self {
        self.auth_code_lifetime = seconds;
        self
    }

    /// Set personal access token lifetime
    pub fn personal_access_token_lifetime(mut self, seconds: Option<i64>) -> Self {
        self.personal_access_token_lifetime = seconds;
        self
    }

    /// Enable or disable PKCE enforcement
    pub fn enforce_pkce(mut self, enforce: bool) -> Self {
        self.enforce_pkce = enforce;
        self
    }

    /// Allow plain text PKCE (not recommended)
    pub fn allow_plain_pkce(mut self, allow: bool) -> Self {
        self.allow_plain_pkce = allow;
        self
    }

    /// Set default scopes
    pub fn default_scopes(mut self, scopes: Vec<String>) -> Self {
        self.default_scopes = scopes;
        self
    }

    /// Enable password grant
    pub fn enable_password_grant(mut self, enable: bool) -> Self {
        self.enable_password_grant = enable;
        self
    }

    /// Enable implicit grant
    pub fn enable_implicit_grant(mut self, enable: bool) -> Self {
        self.enable_implicit_grant = enable;
        self
    }

    /// Enable client credentials grant
    pub fn enable_client_credentials_grant(mut self, enable: bool) -> Self {
        self.enable_client_credentials_grant = enable;
        self
    }

    /// Enable authorization code grant
    pub fn enable_authorization_code_grant(mut self, enable: bool) -> Self {
        self.enable_authorization_code_grant = enable;
        self
    }

    /// Enable refresh token grant
    pub fn enable_refresh_token_grant(mut self, enable: bool) -> Self {
        self.enable_refresh_token_grant = enable;
        self
    }

    /// Require client authentication
    pub fn require_client_authentication(mut self, require: bool) -> Self {
        self.require_client_authentication = require;
        self
    }

    /// Set token length
    pub fn token_length(mut self, length: usize) -> Self {
        self.token_length = length;
        self
    }

    /// Get access token lifetime as Duration
    pub fn access_token_duration(&self) -> Duration {
        Duration::seconds(self.access_token_lifetime)
    }

    /// Get refresh token lifetime as Duration
    pub fn refresh_token_duration(&self) -> Duration {
        Duration::seconds(self.refresh_token_lifetime)
    }

    /// Get auth code lifetime as Duration
    pub fn auth_code_duration(&self) -> Duration {
        Duration::seconds(self.auth_code_lifetime)
    }

    /// Get personal access token lifetime as Duration
    pub fn personal_access_token_duration(&self) -> Option<Duration> {
        self.personal_access_token_lifetime
            .map(Duration::seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PassportConfig::default();
        assert_eq!(config.access_token_lifetime, 3600);
        assert_eq!(config.enforce_pkce, true);
        assert_eq!(config.enable_password_grant, false);
    }

    #[test]
    fn test_builder_pattern() {
        let config = PassportConfig::new()
            .access_token_lifetime(7200)
            .enforce_pkce(false)
            .enable_password_grant(true);

        assert_eq!(config.access_token_lifetime, 7200);
        assert_eq!(config.enforce_pkce, false);
        assert_eq!(config.enable_password_grant, true);
    }

    #[test]
    fn test_duration_conversion() {
        let config = PassportConfig::default();
        let duration = config.access_token_duration();
        assert_eq!(duration.num_seconds(), 3600);
    }
}
