//! Database Configuration
//!
//! Manages database connection settings and provides
//! a connection pool for the application

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use serde::Deserialize;
use std::time::Duration;
use std::path::Path;
use tracing::log;

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: 30,
            idle_timeout: 600,
        }
    }
}

impl DatabaseConfig {
    /// Create a new database connection pool
    ///
    /// For SQLite databases, this will automatically create the database file
    /// if it doesn't exist (Laravel-like behavior)
    pub async fn connect(&self) -> Result<DatabaseConnection, DbErr> {
        // Auto-create SQLite database file if it doesn't exist (like Laravel)
        if self.url.starts_with("sqlite:") && !self.url.contains(":memory:") {
            // Extract file path from sqlite:./path/to/db.sqlite or sqlite://path/to/db.sqlite
            let file_path = self.url
                .trim_start_matches("sqlite:")
                .trim_start_matches("//")
                .trim_start_matches("./");

            let path = Path::new(file_path);

            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        DbErr::Conn(format!("Failed to create database directory: {}", e))
                    })?;
                }
            }

            // Create the database file if it doesn't exist (touch)
            if !path.exists() {
                std::fs::File::create(path).map_err(|e| {
                    DbErr::Conn(format!("Failed to create database file: {}", e))
                })?;
                tracing::info!("📝 Created SQLite database file: {}", file_path);
            }
        }

        let mut opt = ConnectOptions::new(&self.url);
        opt.max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .connect_timeout(Duration::from_secs(self.connect_timeout))
            .idle_timeout(Duration::from_secs(self.idle_timeout))
            .sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Debug);

        Database::connect(opt).await
    }

    /// Load database configuration from environment
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./data.db".to_string()),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            connect_timeout: std::env::var("DB_CONNECT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            idle_timeout: std::env::var("DB_IDLE_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(600),
        }
    }
}
