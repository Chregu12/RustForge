//! Testing utilities for database tests

use crate::{DatabaseConfig, DatabaseManager, DbResult};
use sea_orm::DatabaseConnection;
use std::time::Duration;

/// Test database using SQLite in-memory
///
/// Automatically creates an in-memory SQLite database for testing.
/// The database is dropped when the `TestDatabase` is dropped.
///
/// # Example
///
/// ```rust
/// use rf_orm::testing::TestDatabase;
///
/// #[tokio::test]
/// async fn test_with_database() {
///     let test_db = TestDatabase::new().await;
///
///     // Use test_db.connection() for queries
///     assert!(test_db.ping().await.is_ok());
///
///     // Database automatically cleaned up when test_db is dropped
/// }
/// ```
pub struct TestDatabase {
    manager: DatabaseManager,
}

impl TestDatabase {
    /// Create new test database (SQLite in-memory)
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_orm::testing::TestDatabase;
    ///
    /// # async fn example() {
    /// let test_db = TestDatabase::new().await;
    /// # }
    /// ```
    pub async fn new() -> Self {
        let config = DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
            min_connections: 1,
            connect_timeout: Duration::from_secs(5),
            idle_timeout: None,
            acquire_timeout: Duration::from_secs(5),
            log_queries: false,
            log_level: "off".to_string(),
        };

        let manager = DatabaseManager::connect(config)
            .await
            .expect("Failed to create test database");

        Self { manager }
    }

    /// Get reference to database connection
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_orm::testing::TestDatabase;
    /// # async fn example(test_db: &TestDatabase) {
    /// let conn = test_db.connection();
    /// // Use conn for queries
    /// # }
    /// ```
    pub fn connection(&self) -> &DatabaseConnection {
        self.manager.connection()
    }

    /// Ping database
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_orm::testing::TestDatabase;
    /// # async fn example(test_db: &TestDatabase) -> Result<(), Box<dyn std::error::Error>> {
    /// test_db.ping().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ping(&self) -> DbResult<()> {
        self.manager.ping().await
    }

    /// Get database manager reference
    pub fn manager(&self) -> &DatabaseManager {
        &self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_creation() {
        let test_db = TestDatabase::new().await;
        assert!(test_db.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_connection_access() {
        let test_db = TestDatabase::new().await;
        let conn = test_db.connection();
        assert!(conn.ping().await.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_test_databases() {
        let test_db1 = TestDatabase::new().await;
        let test_db2 = TestDatabase::new().await;

        assert!(test_db1.ping().await.is_ok());
        assert!(test_db2.ping().await.is_ok());
    }
}
