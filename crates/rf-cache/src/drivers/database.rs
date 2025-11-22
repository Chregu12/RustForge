//! Database cache backend driver
//!
//! Provides a database-backed caching solution using SeaORM.

use crate::{Cache, CacheError, CacheResult};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use sea_orm::{
    entity::prelude::*, ActiveModelBehavior, ColumnTrait, DatabaseConnection, EntityTrait,
    QueryFilter, Set,
};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Cache entry entity
pub mod cache_entry {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "cache_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub key: String,
        pub value: Vec<u8>,
        pub expires_at: Option<DateTime<Utc>>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// Database cache driver
pub struct DatabaseDriver {
    db: DatabaseConnection,
}

impl DatabaseDriver {
    /// Create a new database cache driver
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_cache::drivers::database::DatabaseDriver;
    /// use sea_orm::{Database, DatabaseConnection};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = Database::connect("sqlite::memory:").await?;
    /// let driver = DatabaseDriver::new(db);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Clean up expired cache entries
    pub async fn cleanup_expired(&self) -> Result<u64, CacheError> {
        let result = cache_entry::Entity::delete_many()
            .filter(cache_entry::Column::ExpiresAt.lt(Utc::now()))
            .exec(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to clean up expired entries: {}", e)))?;

        Ok(result.rows_affected)
    }
}

#[async_trait]
impl Cache for DatabaseDriver {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        // Clean up expired entries first (probabilistic cleanup)
        if rand::random::<u8>() < 10 {
            // 10/256 chance (~4%)
            let _ = self.cleanup_expired().await;
        }

        // Find the entry
        let entry = cache_entry::Entity::find_by_id(key)
            .one(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to get cache entry: {}", e)))?;

        match entry {
            Some(e) => {
                // Check if expired
                if let Some(expires_at) = e.expires_at {
                    if expires_at < Utc::now() {
                        // Delete expired entry
                        let _ = cache_entry::Entity::delete_by_id(key)
                            .exec(&self.db)
                            .await;
                        return Ok(None);
                    }
                }

                // Deserialize value
                let value = serde_json::from_slice(&e.value)
                    .map_err(|e| CacheError::Deserialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        let expires_at = Utc::now() + ChronoDuration::seconds(ttl.as_secs() as i64);

        // Create active model
        let active = cache_entry::ActiveModel {
            key: Set(key.to_string()),
            value: Set(serialized),
            expires_at: Set(Some(expires_at)),
        };

        // Insert or update
        cache_entry::Entity::insert(active)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(cache_entry::Column::Key)
                    .update_columns([cache_entry::Column::Value, cache_entry::Column::ExpiresAt])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to set cache entry: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        cache_entry::Entity::delete_by_id(key)
            .exec(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to delete cache entry: {}", e)))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let entry = cache_entry::Entity::find_by_id(key)
            .one(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to check cache entry: {}", e)))?;

        match entry {
            Some(e) => {
                // Check if expired
                if let Some(expires_at) = e.expires_at {
                    if expires_at < Utc::now() {
                        // Delete expired entry
                        let _ = cache_entry::Entity::delete_by_id(key)
                            .exec(&self.db)
                            .await;
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn flush(&self) -> CacheResult<()> {
        cache_entry::Entity::delete_many()
            .exec(&self.db)
            .await
            .map_err(|e| CacheError::Backend(format!("Failed to flush cache: {}", e)))?;

        Ok(())
    }
}

/// Migration helper to create the cache_entries table
pub fn get_migration_sql(database_type: &str) -> &'static str {
    match database_type {
        "postgres" | "postgresql" => {
            r#"
CREATE TABLE IF NOT EXISTS cache_entries (
    key VARCHAR(255) PRIMARY KEY,
    value BYTEA NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_cache_expires_at ON cache_entries(expires_at);
"#
        }
        "mysql" => {
            r#"
CREATE TABLE IF NOT EXISTS cache_entries (
    `key` VARCHAR(255) PRIMARY KEY,
    `value` BLOB NOT NULL,
    expires_at TIMESTAMP NULL,
    INDEX idx_cache_expires_at (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
"#
        }
        "sqlite" => {
            r#"
CREATE TABLE IF NOT EXISTS cache_entries (
    key TEXT PRIMARY KEY,
    value BLOB NOT NULL,
    expires_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_cache_expires_at ON cache_entries(expires_at);
"#
        }
        _ => {
            r#"
CREATE TABLE cache_entries (
    key VARCHAR(255) PRIMARY KEY,
    value BLOB NOT NULL,
    expires_at TIMESTAMP NULL
);

CREATE INDEX idx_cache_expires_at ON cache_entries(expires_at);
"#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DatabaseBackend, Schema};

    async fn setup_test_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to test database");

        // Create schema
        let schema = Schema::new(DatabaseBackend::Sqlite);
        let stmt = schema.create_table_from_entity(cache_entry::Entity);

        db.execute(db.get_database_backend().build(&stmt))
            .await
            .expect("Failed to create test table");

        db
    }

    #[tokio::test]
    async fn test_database_driver_basic_operations() {
        let db = setup_test_db().await;
        let driver = DatabaseDriver::new(db);

        // Test set and get
        driver
            .set("test_key", &"test_value", Duration::from_secs(60))
            .await
            .unwrap();

        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test exists
        assert!(driver.exists("test_key").await.unwrap());

        // Test delete
        driver.delete("test_key").await.unwrap();
        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, None);
        assert!(!driver.exists("test_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_database_driver_expiration() {
        let db = setup_test_db().await;
        let driver = DatabaseDriver::new(db);

        // Set with short TTL
        driver
            .set("expire_key", &"value", Duration::from_millis(100))
            .await
            .unwrap();

        let value: Option<String> = driver.get("expire_key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        let value: Option<String> = driver.get("expire_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_database_driver_flush() {
        let db = setup_test_db().await;
        let driver = DatabaseDriver::new(db);

        // Set multiple keys
        driver
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        driver
            .set("key2", &"value2", Duration::from_secs(60))
            .await
            .unwrap();

        // Flush all
        driver.flush().await.unwrap();

        // Verify all deleted
        assert!(!driver.exists("key1").await.unwrap());
        assert!(!driver.exists("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_database_driver_cleanup_expired() {
        let db = setup_test_db().await;
        let driver = DatabaseDriver::new(db);

        // Set keys with different TTLs
        driver
            .set("long_key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        driver
            .set("short_key", &"value", Duration::from_millis(50))
            .await
            .unwrap();

        // Wait for short key to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Clean up expired
        let removed = driver.cleanup_expired().await.unwrap();
        assert_eq!(removed, 1);

        // Verify long key still exists
        assert!(driver.exists("long_key").await.unwrap());
        assert!(!driver.exists("short_key").await.unwrap());
    }
}
