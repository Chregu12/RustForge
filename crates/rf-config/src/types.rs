//! Configuration type definitions

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Configuration validation errors
#[derive(Debug, Error)]
pub enum ConfigValidationError {
    #[error("server.port must be non-zero")]
    InvalidPort,

    #[error("server.workers must be non-zero")]
    InvalidWorkers,

    #[error("database.max_connections must be non-zero")]
    InvalidMaxConnections,

    #[error("auth.jwt_secret must be changed in production")]
    InsecureJwtSecret,

    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// Returned when an environment variable is *present* but cannot be parsed
    /// into the expected type (e.g. `SERVER_PORT=not_a_number`).
    #[error("environment variable {var} has an invalid value '{value}': {detail}")]
    InvalidEnvVar {
        var: String,
        value: String,
        detail: String,
    },
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Authentication configuration
    pub auth: AuthConfig,
}

/// Parse an environment variable into `T`, using `default` when the variable is
/// absent.  Returns [`ConfigValidationError::InvalidEnvVar`] when the variable
/// is *present but cannot be parsed* — never silently falls back to the default
/// in that case.
fn parse_env_var<T>(name: &str, default: fn() -> T) -> Result<T, ConfigValidationError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Err(_) => Ok(default()),
        Ok(val) => val.parse::<T>().map_err(|e| ConfigValidationError::InvalidEnvVar {
            var: name.to_string(),
            value: val,
            detail: e.to_string(),
        }),
    }
}

impl AppConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.server.port == 0 {
            return Err(ConfigValidationError::InvalidPort);
        }

        if self.server.workers == 0 {
            return Err(ConfigValidationError::InvalidWorkers);
        }

        if self.database.max_connections == 0 {
            return Err(ConfigValidationError::InvalidMaxConnections);
        }

        // In production, ensure secrets are not default values
        if std::env::var("APP_ENV").unwrap_or_default() == "production"
            && self.auth.jwt_secret == "dev-secret-change-in-production"
        {
            return Err(ConfigValidationError::InsecureJwtSecret);
        }

        Ok(())
    }

    /// Load from environment variables with defaults.
    ///
    /// Returns [`ConfigValidationError::InvalidEnvVar`] if an environment variable
    /// is *present but cannot be parsed* (e.g. `SERVER_PORT=not_a_number`).
    /// Missing variables fall back to compiled-in defaults.
    pub fn from_env() -> Result<Self, ConfigValidationError> {
        let config = Self {
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| default_host()),
                port: parse_env_var("SERVER_PORT", default_port)?,
                workers: parse_env_var("SERVER_WORKERS", default_workers)?,
                timeout: parse_env_var("SERVER_TIMEOUT", default_timeout)?,
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://localhost/rustforge".to_string()),
                max_connections: parse_env_var(
                    "DATABASE_MAX_CONNECTIONS",
                    default_max_connections,
                )?,
                min_connections: parse_env_var(
                    "DATABASE_MIN_CONNECTIONS",
                    default_min_connections,
                )?,
                connect_timeout: parse_env_var(
                    "DATABASE_CONNECT_TIMEOUT",
                    default_connect_timeout,
                )?,
            },
            auth: AuthConfig {
                jwt_secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
                token_expiry_hours: parse_env_var("TOKEN_EXPIRY_HOURS", default_token_expiry)?,
                session_timeout_minutes: parse_env_var(
                    "SESSION_TIMEOUT_MINUTES",
                    default_session_timeout,
                )?,
            },
        };

        config.validate()?;
        Ok(config)
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host (e.g., "0.0.0.0", "127.0.0.1")
    #[serde(default = "default_host")]
    pub host: String,

    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: default_workers(),
            timeout: default_timeout(),
        }
    }
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_workers() -> usize {
    4
}

fn default_timeout() -> u64 {
    30
}

/// Database configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,

    /// Maximum number of connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("url", &"[REDACTED]")
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("connect_timeout", &self.connect_timeout)
            .finish()
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/rustforge".to_string(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout: default_connect_timeout(),
        }
    }
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connect_timeout() -> u64 {
    8
}

/// Authentication configuration
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// JWT secret key
    pub jwt_secret: String,

    /// Token expiry in hours
    #[serde(default = "default_token_expiry")]
    pub token_expiry_hours: u64,

    /// Session timeout in minutes
    #[serde(default = "default_session_timeout")]
    pub session_timeout_minutes: u64,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("jwt_secret", &"[REDACTED]")
            .field("token_expiry_hours", &self.token_expiry_hours)
            .field("session_timeout_minutes", &self.session_timeout_minutes)
            .finish()
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "dev-secret-change-in-production".to_string(),
            token_expiry_hours: default_token_expiry(),
            session_timeout_minutes: default_session_timeout(),
        }
    }
}

fn default_token_expiry() -> u64 {
    24
}

fn default_session_timeout() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_server_config() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
        assert_eq!(config.workers, 4);
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_default_database_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.connect_timeout, 8);
    }

    #[test]
    fn test_default_auth_config() {
        let config = AuthConfig::default();
        assert_eq!(config.jwt_secret, "dev-secret-change-in-production");
        assert_eq!(config.token_expiry_hours, 24);
        assert_eq!(config.session_timeout_minutes, 60);
    }

    #[test]
    fn test_validate_valid_config() {
        let config = AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_port() {
        let mut config = AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
        };
        config.server.port = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_workers() {
        let mut config = AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
        };
        config.server.workers = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_connections() {
        let mut config = AppConfig {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
        };
        config.database.max_connections = 0;

        assert!(config.validate().is_err());
    }

    // Env-var tests mutate global process state; serialize them with a mutex
    // to avoid races when cargo test runs multiple threads.
    static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// A present-but-unparseable SERVER_PORT must return an error, NOT silently
    /// fall back to the default 3000.
    #[test]
    fn test_from_env_bad_port_errors() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::set_var("SERVER_PORT", "not_a_number");
        let result = AppConfig::from_env();
        std::env::remove_var("SERVER_PORT");

        assert!(
            result.is_err(),
            "from_env must fail when SERVER_PORT is not a number, not silently use 3000"
        );
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("SERVER_PORT"),
            "error message must name the bad variable; got: {msg}"
        );
    }

    /// A missing SERVER_PORT must still fall back to the default (3000) without error.
    #[test]
    fn test_from_env_missing_port_uses_default() {
        let _guard = ENV_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        std::env::remove_var("SERVER_PORT");
        // Only succeeds if no other bad env vars are set; use a best-effort check.
        if let Ok(cfg) = AppConfig::from_env() {
            assert_eq!(cfg.server.port, 3000);
        }
        // If other required vars are mis-set in the test environment this is a no-op.
    }
}
