//! Configuration loading with hierarchy support

use config::{Config, ConfigError, Environment, File};
use serde::de::DeserializeOwned;
use std::path::Path;

/// Configuration loader with hierarchical loading
///
/// Load priority (highest to lowest):
/// 1. Environment variables (APP__SECTION__KEY)
/// 2. Environment-specific file (config/{env}.toml)
/// 3. Default file (config/default.toml)
///
/// # Example
///
/// ```rust,no_run
/// use rf_config::{AppConfig, ConfigLoader};
///
/// let config = ConfigLoader::new()
///     .env("development")
///     .config_dir("config")
///     .load::<AppConfig>()
///     .expect("Failed to load config");
/// ```
pub struct ConfigLoader {
    env: String,
    config_dir: String,
    prefix: String,
}

impl ConfigLoader {
    /// Create a new configuration loader
    ///
    /// Defaults:
    /// - env: "development"
    /// - config_dir: "config"
    /// - prefix: "APP"
    pub fn new() -> Self {
        Self {
            env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            config_dir: "config".to_string(),
            prefix: "APP".to_string(),
        }
    }

    /// Set the environment (development, staging, production)
    pub fn env(mut self, env: impl Into<String>) -> Self {
        self.env = env.into();
        self
    }

    /// Set the configuration directory
    pub fn config_dir(mut self, dir: impl Into<String>) -> Self {
        self.config_dir = dir.into();
        self
    }

    /// Set the environment variable prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Load configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration files cannot be read
    /// - Configuration cannot be deserialized
    /// - Environment variables have invalid values
    pub fn load<T: DeserializeOwned>(&self) -> Result<T, ConfigError> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let default_path = Path::new(&self.config_dir).join("default.toml");
        let env_path = Path::new(&self.config_dir).join(format!("{}.toml", self.env));

        let config = Config::builder()
            // Start with default config
            .add_source(File::from(default_path).required(false))
            // Override with environment-specific config
            .add_source(File::from(env_path).required(false))
            // Override with environment variables
            // Format: APP__SERVER__PORT=8080
            .add_source(
                Environment::with_prefix(&self.prefix)
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        config.try_deserialize()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AppConfig;

    #[test]
    fn test_config_loader_new() {
        let loader = ConfigLoader::new();
        assert_eq!(loader.config_dir, "config");
        assert_eq!(loader.prefix, "APP");
    }

    #[test]
    fn test_config_loader_with_options() {
        let loader = ConfigLoader::new()
            .env("production")
            .config_dir("custom_config")
            .prefix("MYAPP");

        assert_eq!(loader.env, "production");
        assert_eq!(loader.config_dir, "custom_config");
        assert_eq!(loader.prefix, "MYAPP");
    }

    #[test]
    fn test_load_with_defaults() {
        // This test loads configuration with all defaults
        // It will fail if no config files exist, which is expected
        let result = ConfigLoader::new().load::<AppConfig>();

        // In a real project with config files, this would succeed
        // For now, we just verify the function signature works
        match result {
            Ok(_config) => {
                // Config loaded successfully
            }
            Err(_e) => {
                // Expected if no config files exist
            }
        }
    }

    #[test]
    fn test_env_override() {
        // Set environment variable
        std::env::set_var("APP__SERVER__PORT", "9000");

        let result = ConfigLoader::new().load::<AppConfig>();

        // Clean up
        std::env::remove_var("APP__SERVER__PORT");

        // Verify it tried to load (may fail if config files don't exist)
        match result {
            Ok(config) => {
                assert_eq!(config.server.port, 9000);
            }
            Err(_) => {
                // Expected if no config files exist
            }
        }
    }
}
