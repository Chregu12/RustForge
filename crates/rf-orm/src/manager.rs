//! Database connection manager

use crate::{DatabaseConfig, DbError, DbResult};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{debug, info};

/// Database connection manager
///
/// Manages database connection pool and provides access to the connection.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::{DatabaseManager, DatabaseConfig};
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = DatabaseConfig {
///     url: "sqlite::memory:".to_string(),
///     max_connections: 10,
///     min_connections: 2,
///     connect_timeout: Duration::from_secs(8),
///     idle_timeout: Some(Duration::from_secs(600)),
///     acquire_timeout: Duration::from_secs(30),
///     log_queries: false,
///     log_level: "info".to_string(),
/// };
///
/// let db = DatabaseManager::connect(config).await?;
///
/// // Use connection
/// let conn = db.connection();
///
/// // Health check
/// db.ping().await?;
///
/// // Close
/// db.close().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct DatabaseManager {
    connection: DatabaseConnection,
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Connect to database with given configuration
    ///
    /// # Errors
    ///
    /// Returns `DbError::ConnectionFailed` if connection fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::{DatabaseManager, DatabaseConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = DatabaseConfig::default();
    /// let db = DatabaseManager::connect(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(config: DatabaseConfig) -> DbResult<Self> {
        info!("Connecting to database: {}", mask_password(&config.url));

        // Build connection options
        let mut opt = ConnectOptions::new(&config.url);
        opt.max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(config.connect_timeout)
            .acquire_timeout(config.acquire_timeout);

        if let Some(idle_timeout) = config.idle_timeout {
            opt.idle_timeout(idle_timeout);
        }

        if config.log_queries {
            opt.sqlx_logging(true)
                .sqlx_logging_level(parse_log_level(&config.log_level));
        } else {
            opt.sqlx_logging(false);
        }

        // Connect
        let connection = Database::connect(opt)
            .await
            .map_err(|source| DbError::ConnectionFailed { source })?;

        debug!("Database connection established");

        Ok(Self { connection, config })
    }

    /// Create database manager from rf-config DatabaseConfig
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_config::{ConfigLoader, AppConfig};
    /// use rf_orm::DatabaseManager;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let app_config = ConfigLoader::new().load::<AppConfig>()?;
    /// let db = DatabaseManager::from_config(&app_config.database).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_config(config: &rf_config::DatabaseConfig) -> DbResult<Self> {
        let db_config = DatabaseConfig {
            url: config.url.clone(),
            max_connections: config.max_connections,
            min_connections: config.min_connections,
            connect_timeout: Duration::from_secs(config.connect_timeout),
            idle_timeout: None, // Not in rf-config yet
            acquire_timeout: Duration::from_secs(30),
            log_queries: false,
            log_level: "info".to_string(),
        };

        Self::connect(db_config).await
    }

    /// Get reference to database connection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::DatabaseManager;
    /// # async fn example(db: &DatabaseManager) {
    /// let conn = db.connection();
    /// // Use conn for queries
    /// # }
    /// ```
    pub fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Get configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Ping database to check connection health
    ///
    /// # Errors
    ///
    /// Returns error if database is unreachable
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::DatabaseManager;
    /// # async fn example(db: &DatabaseManager) -> Result<(), Box<dyn std::error::Error>> {
    /// db.ping().await?;
    /// println!("Database is healthy");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ping(&self) -> DbResult<()> {
        self.connection
            .ping()
            .await
            .map_err(|source| DbError::ConnectionFailed { source })
    }

    /// Close database connection gracefully
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::DatabaseManager;
    /// # async fn example(db: DatabaseManager) -> Result<(), Box<dyn std::error::Error>> {
    /// db.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(self) -> DbResult<()> {
        info!("Closing database connection");
        self.connection
            .close()
            .await
            .map_err(|source| DbError::ConnectionFailed { source })?;
        debug!("Database connection closed");
        Ok(())
    }
}

/// Mask password in database URL for logging
fn mask_password(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(scheme_pos) = url.find("://") {
                let scheme_end = scheme_pos + 3;
                if colon_pos > scheme_end {
                    let mut masked = String::from(&url[..colon_pos + 1]);
                    masked.push_str("****");
                    masked.push_str(&url[at_pos..]);
                    return masked;
                }
            }
        }
    }
    url.to_string()
}

/// Parse log level string to tracing level
fn parse_log_level(level: &str) -> log::LevelFilter {
    match level.to_lowercase().as_str() {
        "off" => log::LevelFilter::Off,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_password() {
        let url = "postgres://user:secret@localhost/db";
        let masked = mask_password(url);
        assert!(masked.contains("****"));
        assert!(!masked.contains("secret"));
        assert!(masked.contains("user"));
        assert!(masked.contains("@localhost/db"));
    }

    #[test]
    fn test_mask_password_no_password() {
        let url = "sqlite::memory:";
        let masked = mask_password(url);
        assert_eq!(masked, url);
    }

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("debug"), log::LevelFilter::Debug);
        assert_eq!(parse_log_level("info"), log::LevelFilter::Info);
        assert_eq!(parse_log_level("error"), log::LevelFilter::Error);
        assert_eq!(parse_log_level("invalid"), log::LevelFilter::Info);
    }

    #[tokio::test]
    async fn test_connect_sqlite_memory() {
        let config = DatabaseConfig::default();
        let db = DatabaseManager::connect(config).await;
        assert!(db.is_ok());

        let db = db.unwrap();
        assert!(db.ping().await.is_ok());

        assert!(db.close().await.is_ok());
    }

    #[tokio::test]
    async fn test_connection_reference() {
        let config = DatabaseConfig::default();
        let db = DatabaseManager::connect(config).await.unwrap();

        let conn = db.connection();
        assert!(conn.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_connection() {
        let config = DatabaseConfig {
            url: "invalid://connection".to_string(),
            ..Default::default()
        };

        let result = DatabaseManager::connect(config).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            DbError::ConnectionFailed { .. } => {}
            _ => panic!("Expected ConnectionFailed error"),
        }
    }
}
