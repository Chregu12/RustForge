//! Application Settings
//!
//! Centralized configuration management for the application.
//! Loads settings from environment variables and .env file.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub app: AppConfig,
    pub server: ServerConfig,
    pub database: super::DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub debug: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl Settings {
    /// Load settings from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        Ok(Self {
            app: AppConfig {
                name: std::env::var("APP_NAME").unwrap_or_else(|_| "RustForge App".to_string()),
                env: std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()),
                debug: std::env::var("APP_DEBUG")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(true),
            },
            server: ServerConfig {
                host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: std::env::var("PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3000),
                cors_enabled: std::env::var("CORS_ENABLED")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(true),
            },
            database: super::DatabaseConfig::from_env(),
            jwt: JwtConfig {
                secret: std::env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "change-this-secret-in-production-min-32-chars".to_string()),
                expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(24),
            },
        })
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.app.env == "development"
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.app.env == "production"
    }
}
