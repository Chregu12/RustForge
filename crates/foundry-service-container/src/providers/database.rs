use async_trait::async_trait;
use crate::container::Container;
use crate::error::Result;
use crate::provider::ServiceProvider;

/// Database service provider for database connections and configurations
pub struct DatabaseServiceProvider;

impl DatabaseServiceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceProvider for DatabaseServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register database driver
        container
            .singleton("database.driver", || {
                Ok(std::env::var("DB_DRIVER").unwrap_or_else(|_| "postgres".to_string()))
            })
            .await?;

        // Register database connection URL
        container
            .singleton("database.url", || {
                Ok(std::env::var("DATABASE_URL").unwrap_or_default())
            })
            .await?;

        // Register individual database connection parameters
        container
            .singleton("database.host", || {
                Ok(std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()))
            })
            .await?;

        container
            .singleton("database.port", || {
                Ok(std::env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse::<u16>()
                    .unwrap_or(5432))
            })
            .await?;

        container
            .singleton("database.name", || {
                Ok(std::env::var("DB_DATABASE").unwrap_or_else(|_| "foundry".to_string()))
            })
            .await?;

        container
            .singleton("database.username", || {
                Ok(std::env::var("DB_USERNAME").unwrap_or_else(|_| "postgres".to_string()))
            })
            .await?;

        container
            .singleton("database.password", || {
                Ok(std::env::var("DB_PASSWORD").unwrap_or_default())
            })
            .await?;

        // Register connection pool settings
        container
            .singleton("database.pool.max_connections", || {
                Ok(std::env::var("DB_POOL_MAX")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse::<u32>()
                    .unwrap_or(10))
            })
            .await?;

        container
            .singleton("database.pool.min_connections", || {
                Ok(std::env::var("DB_POOL_MIN")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse::<u32>()
                    .unwrap_or(1))
            })
            .await?;

        container
            .singleton("database.pool.timeout", || {
                Ok(std::env::var("DB_POOL_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse::<u64>()
                    .unwrap_or(30))
            })
            .await?;

        // Register SQLite specific settings
        container
            .singleton("database.sqlite.path", || {
                Ok(std::env::var("SQLITE_PATH")
                    .unwrap_or_else(|_| "storage/database.sqlite".to_string()))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        // Could initialize database connections here
        Ok(())
    }

    fn name(&self) -> &str {
        "DatabaseServiceProvider"
    }

    fn defer(&self) -> Vec<String> {
        // Database services can be lazy loaded
        vec![
            "database.driver".to_string(),
            "database.url".to_string(),
            "database.host".to_string(),
            "database.port".to_string(),
            "database.name".to_string(),
            "database.username".to_string(),
            "database.password".to_string(),
            "database.pool.max_connections".to_string(),
            "database.pool.min_connections".to_string(),
            "database.pool.timeout".to_string(),
            "database.sqlite.path".to_string(),
        ]
    }
}

impl Default for DatabaseServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
