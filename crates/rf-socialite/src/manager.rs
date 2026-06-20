//! Socialite manager for provider registration and management
//!
//! This module provides a centralized manager for OAuth2 providers,
//! similar to Laravel's Socialite facade.

use crate::config::SocialiteConfig;
use crate::driver::{Driver, DriverBuilder, SocialiteError};
use crate::providers::Provider;
use crate::state::StateManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Socialite manager
///
/// Manages OAuth2 providers and their configurations
#[derive(Clone)]
pub struct SocialiteManager {
    config: SocialiteConfig,
    state_manager: StateManager,
    drivers: Arc<Mutex<HashMap<String, Driver>>>,
}

impl SocialiteManager {
    /// Create a new Socialite manager
    pub fn new(config: SocialiteConfig) -> Self {
        Self {
            config,
            state_manager: StateManager::new(),
            drivers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::new(SocialiteConfig::from_env())
    }

    /// Get a driver for the specified provider
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_socialite::manager::SocialiteManager;
    ///
    /// let manager = SocialiteManager::from_env();
    /// let driver = manager.driver("github").unwrap();
    /// ```
    pub fn driver(&self, name: &str) -> Result<DriverBuilder, SocialiteError> {
        let config = self.config.get_provider(name).ok_or_else(|| {
            SocialiteError::InvalidConfig(format!("Provider '{}' not configured", name))
        })?;

        let provider = self.name_to_provider(name)?;

        let mut builder = Driver::new(provider)
            .client_id(&config.client_id)
            .client_secret(&config.client_secret)
            .redirect_url(&config.redirect_uri);

        if !config.scopes.is_empty() {
            builder = builder.scopes(config.scopes.clone());
        }

        Ok(builder)
    }

    /// Get Google driver
    pub fn google(&self) -> Result<DriverBuilder, SocialiteError> {
        self.driver("google")
    }

    /// Get GitHub driver
    pub fn github(&self) -> Result<DriverBuilder, SocialiteError> {
        self.driver("github")
    }

    /// Get Facebook driver
    pub fn facebook(&self) -> Result<DriverBuilder, SocialiteError> {
        self.driver("facebook")
    }

    /// Get Twitter driver
    pub fn twitter(&self) -> Result<DriverBuilder, SocialiteError> {
        self.driver("twitter")
    }

    /// Get state manager
    pub fn state_manager(&self) -> &StateManager {
        &self.state_manager
    }

    /// Generate a state token for CSRF protection
    pub fn generate_state(&self) -> String {
        self.state_manager.generate()
    }

    /// Verify a state token
    pub fn verify_state(&self, state: &str) -> bool {
        self.state_manager.verify(state)
    }

    /// Convert provider name to Provider enum
    fn name_to_provider(&self, name: &str) -> Result<Provider, SocialiteError> {
        match name.to_lowercase().as_str() {
            "google" => Ok(Provider::Google),
            "github" => Ok(Provider::GitHub),
            "facebook" => Ok(Provider::Facebook),
            "twitter" => Ok(Provider::Twitter),
            _ => Err(SocialiteError::InvalidConfig(format!(
                "Unknown provider: {}",
                name
            ))),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &SocialiteConfig {
        &self.config
    }
}

impl Default for SocialiteManager {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let config = SocialiteConfig::new();
        let manager = SocialiteManager::new(config);
        assert!(manager.config().google.is_none());
    }

    #[test]
    fn test_name_to_provider() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        assert!(manager.name_to_provider("google").is_ok());
        assert!(manager.name_to_provider("github").is_ok());
        assert!(manager.name_to_provider("invalid").is_err());
    }

    #[test]
    fn test_state_generation() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let state = manager.generate_state();
        assert!(!state.is_empty());
    }

    #[test]
    fn test_state_verification() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let state = manager.generate_state();
        assert!(manager.verify_state(&state));
        // Should be one-time use
        assert!(!manager.verify_state(&state));
    }

    #[test]
    fn test_driver_without_config() {
        let manager = SocialiteManager::new(SocialiteConfig::new());
        let result = manager.driver("google");
        assert!(result.is_err());
    }

    #[test]
    fn test_driver_with_config() {
        let config = SocialiteConfig::new().with_google(ProviderConfig::new("id", "secret", "uri"));
        let manager = SocialiteManager::new(config);
        let result = manager.driver("google");
        assert!(result.is_ok());
    }
}
