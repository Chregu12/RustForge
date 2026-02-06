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

    /// Load from environment variables with defaults
    pub fn from_env() -> Result<Self, ConfigValidationError> {
        let config = Self {
            server: ServerConfig {
                host: std::env::var("SERVER_HOST").unwrap_or_else(|_| default_host()),
                port: std::env::var("SERVER_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(default_port),
                workers: std::env::var("SERVER_WORKERS")
                    .ok()
                    .and_then(|w| w.parse().ok())
                    .unwrap_or_else(default_workers),
                timeout: std::env::var("SERVER_TIMEOUT")
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or_else(default_timeout),
            },
            database: DatabaseConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgres://localhost/rustforge".to_string()),
                max_connections: std::env::var("DATABASE_MAX_CONNECTIONS")
                    .ok()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or_else(default_max_connections),
                min_connections: std::env::var("DATABASE_MIN_CONNECTIONS")
                    .ok()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or_else(default_min_connections),
                connect_timeout: std::env::var("DATABASE_CONNECT_TIMEOUT")
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or_else(default_connect_timeout),
            },
            auth: AuthConfig {
                jwt_secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
                token_expiry_hours: std::env::var("TOKEN_EXPIRY_HOURS")
                    .ok()
                    .and_then(|e| e.parse().ok())
                    .unwrap_or_else(default_token_expiry),
                session_timeout_minutes: std::env::var("SESSION_TIMEOUT_MINUTES")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(default_session_timeout),
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
