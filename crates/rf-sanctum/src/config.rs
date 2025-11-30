//! Sanctum configuration

use chrono::Duration;
use serde::{Deserialize, Serialize};

/// Configuration for Sanctum authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctumConfig {
    /// Default token expiration duration (None = no expiration)
    pub default_expiration: Option<Duration>,

    /// Token prefix (e.g., "sanctum_")
    pub token_prefix: Option<String>,

    /// Stateful domains for SPA authentication
    /// These domains will use cookie-based auth instead of token auth
    pub stateful_domains: Vec<String>,

    /// Enable token pruning on cleanup
    pub enable_pruning: bool,

    /// Days to keep unused tokens before pruning
    pub prune_after_days: u32,

    /// Track device information (user agent, IP)
    pub track_devices: bool,

    /// Allow transient (non-persistent) tokens
    pub allow_transient_tokens: bool,

    /// CSRF cookie name for SPA authentication
    pub csrf_cookie: String,

    /// Token hash algorithm (currently only SHA256 supported)
    pub hash_algorithm: HashAlgorithm,
}

/// Supported hash algorithms for tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256 hashing (recommended)
    SHA256,
}

impl Default for SanctumConfig {
    fn default() -> Self {
        Self {
            default_expiration: None, // No expiration by default
            token_prefix: Some("sanctum_".to_string()),
            stateful_domains: vec!["localhost".to_string()],
            enable_pruning: true,
            prune_after_days: 90,
            track_devices: true,
            allow_transient_tokens: true,
            csrf_cookie: "XSRF-TOKEN".to_string(),
            hash_algorithm: HashAlgorithm::SHA256,
        }
    }
}

impl SanctumConfig {
    /// Create a new configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set default token expiration
    pub fn with_expiration(mut self, duration: Duration) -> Self {
        self.default_expiration = Some(duration);
        self
    }

    /// Set token prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.token_prefix = Some(prefix.into());
        self
    }

    /// Add a stateful domain for SPA auth
    pub fn add_stateful_domain(mut self, domain: impl Into<String>) -> Self {
        self.stateful_domains.push(domain.into());
        self
    }

    /// Set stateful domains
    pub fn with_stateful_domains(mut self, domains: Vec<String>) -> Self {
        self.stateful_domains = domains;
        self
    }

    /// Enable or disable token pruning
    pub fn with_pruning(mut self, enabled: bool) -> Self {
        self.enable_pruning = enabled;
        self
    }

    /// Set the number of days before pruning unused tokens
    pub fn with_prune_after_days(mut self, days: u32) -> Self {
        self.prune_after_days = days;
        self
    }

    /// Enable or disable device tracking
    pub fn with_device_tracking(mut self, enabled: bool) -> Self {
        self.track_devices = enabled;
        self
    }

    /// Enable or disable transient tokens
    pub fn with_transient_tokens(mut self, enabled: bool) -> Self {
        self.allow_transient_tokens = enabled;
        self
    }

    /// Set CSRF cookie name
    pub fn with_csrf_cookie(mut self, name: impl Into<String>) -> Self {
        self.csrf_cookie = name.into();
        self
    }

    /// Check if a domain is stateful (uses SPA cookie auth)
    pub fn is_stateful_domain(&self, domain: &str) -> bool {
        self.stateful_domains.iter().any(|d| d == domain)
    }

    /// Format a token with the configured prefix
    pub fn format_token(&self, token: &str) -> String {
        if let Some(prefix) = &self.token_prefix {
            format!("{}{}", prefix, token)
        } else {
            token.to_string()
        }
    }

    /// Strip prefix from a token
    pub fn strip_prefix<'a>(&self, token: &'a str) -> &'a str {
        if let Some(prefix) = &self.token_prefix {
            token.strip_prefix(prefix.as_str()).unwrap_or(token)
        } else {
            token
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SanctumConfig::default();
        assert_eq!(config.token_prefix, Some("sanctum_".to_string()));
        assert!(config.enable_pruning);
        assert_eq!(config.prune_after_days, 90);
        assert!(config.track_devices);
    }

    #[test]
    fn test_config_builder() {
        let config = SanctumConfig::new()
            .with_prefix("myapp_")
            .with_prune_after_days(30)
            .with_device_tracking(false);

        assert_eq!(config.token_prefix, Some("myapp_".to_string()));
        assert_eq!(config.prune_after_days, 30);
        assert!(!config.track_devices);
    }

    #[test]
    fn test_stateful_domains() {
        let config = SanctumConfig::new()
            .add_stateful_domain("example.com")
            .add_stateful_domain("app.example.com");

        assert!(config.is_stateful_domain("example.com"));
        assert!(config.is_stateful_domain("app.example.com"));
        assert!(!config.is_stateful_domain("other.com"));
    }

    #[test]
    fn test_token_prefix() {
        let config = SanctumConfig::new().with_prefix("test_");

        let formatted = config.format_token("abc123");
        assert_eq!(formatted, "test_abc123");

        let stripped = config.strip_prefix("test_abc123");
        assert_eq!(stripped, "abc123");
    }

    #[test]
    fn test_no_prefix() {
        let config = SanctumConfig::new();
        let mut config = config;
        config.token_prefix = None;

        let formatted = config.format_token("abc123");
        assert_eq!(formatted, "abc123");

        let stripped = config.strip_prefix("abc123");
        assert_eq!(stripped, "abc123");
    }
}
