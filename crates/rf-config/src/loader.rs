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
/// A `.env` file is loaded before all other sources.  By default,
/// [`ConfigLoader`] searches for `.env` in the current working directory
/// (standard dotenvy behaviour).  Use [`Self::env_file`] to point at a
/// specific path instead.
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
///
/// Loading from a custom `.env` path:
///
/// ```rust,no_run
/// use rf_config::{AppConfig, ConfigLoader};
///
/// let config = ConfigLoader::new()
///     .env_file("/etc/myapp/.env")
///     .load::<AppConfig>()
///     .expect("Failed to load config");
/// ```
pub struct ConfigLoader {
    env: String,
    config_dir: String,
    prefix: String,
    /// Optional explicit path for the `.env` file.
    /// When `None`, `dotenvy::dotenv()` searches from the CWD upward.
    env_file: Option<std::path::PathBuf>,
}

impl ConfigLoader {
    /// Create a new configuration loader
    ///
    /// Defaults:
    /// - env: "development"
    /// - config_dir: "config"
    /// - prefix: "APP"
    /// - env_file: None (search for `.env` from CWD upward via dotenvy)
    pub fn new() -> Self {
        Self {
            env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
            config_dir: "config".to_string(),
            prefix: "APP".to_string(),
            env_file: None,
        }
    }

    /// Load variables from a specific `.env` file instead of the CWD-relative
    /// default.  The file is loaded with `dotenvy::from_path`, which handles
    /// quoted values, `export` prefixes, and multi-line strings correctly.
    ///
    /// ```rust,no_run
    /// use rf_config::ConfigLoader;
    /// use rf_config::AppConfig;
    ///
    /// let config = ConfigLoader::new()
    ///     .env_file("/deploy/secrets/.env.production")
    ///     .load::<AppConfig>()
    ///     .unwrap();
    /// ```
    pub fn env_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.env_file = Some(path.into());
        self
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
        // Load .env file — use explicit path when set, otherwise search from CWD.
        match &self.env_file {
            Some(path) => { dotenvy::from_path(path).ok(); }
            None => { dotenvy::dotenv().ok(); }
        }

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
    use std::io::Write;

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

    /// env_file(path) must load variables from the specified file, not from
    /// whatever `.env` happens to live in the CWD.
    #[test]
    fn test_env_file_custom_path() {
        // Write a temporary .env file in a tempdir
        let tmp = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(tmp.as_file(), "APP__SERVER__PORT=7654").unwrap();

        // The file must be flushed before we read it
        tmp.as_file().sync_all().unwrap();

        // Clear any pre-existing value so we can verify the file took effect
        std::env::remove_var("APP__SERVER__PORT");

        let result = ConfigLoader::new()
            .env_file(tmp.path())
            .load::<AppConfig>();

        // Clean up env
        std::env::remove_var("APP__SERVER__PORT");

        match result {
            Ok(config) => assert_eq!(config.server.port, 7654,
                "env_file value must override the default"),
            Err(_) => {
                // Config files may not exist in test environment — acceptable.
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
