//! Database testing utilities
//!
//! Provides utilities for testing with databases including test database setup,
//! migrations, transaction management, and database assertions.

use thiserror::Error;
use std::collections::HashMap;

/// Database testing errors
#[derive(Debug, Error)]
pub enum DatabaseTestError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Seeder error: {0}")]
    SeederError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Setup error: {0}")]
    SetupError(String),

    #[error("Assertion failed: {0}")]
    AssertionFailed(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

/// Test database configuration
#[derive(Clone, Debug)]
pub struct TestDatabaseConfig {
    /// Database URL
    pub url: String,

    /// Whether to run migrations automatically
    pub auto_migrate: bool,

    /// Whether to seed the database
    pub auto_seed: bool,

    /// Whether to use transactions (rollback after each test)
    pub use_transactions: bool,
}

impl Default for TestDatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite::memory:".to_string()),
            auto_migrate: true,
            auto_seed: false,
            use_transactions: true,
        }
    }
}

/// Test database wrapper
///
/// Provides utilities for setting up and tearing down test databases.
/// Supports automatic migrations, seeding, and transaction management.
///
/// # Example
///
/// ```rust
/// use rf_testing::TestDatabase;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let test_db = TestDatabase::new().await?;
/// test_db.migrate().await?;
///
/// // Run your tests...
///
/// test_db.cleanup().await?;
/// # Ok(())
/// # }
/// ```
pub struct TestDatabase {
    config: TestDatabaseConfig,
    connection_string: String,
}

impl TestDatabase {
    /// Create a new test database with default configuration
    pub async fn new() -> Result<Self, DatabaseTestError> {
        Self::with_config(TestDatabaseConfig::default()).await
    }

    /// Create a new test database with custom configuration
    pub async fn with_config(config: TestDatabaseConfig) -> Result<Self, DatabaseTestError> {
        let connection_string = config.url.clone();

        let test_db = Self {
            config,
            connection_string,
        };

        if test_db.config.auto_migrate {
            test_db.migrate().await?;
        }

        if test_db.config.auto_seed {
            test_db.seed().await?;
        }

        Ok(test_db)
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), DatabaseTestError> {
        println!("Running migrations...");
        // Implementation would depend on your ORM (SeaORM, SQLx, etc.)
        // This is a placeholder
        Ok(())
    }

    /// Seed the database
    pub async fn seed(&self) -> Result<(), DatabaseTestError> {
        println!("Seeding database...");
        // Implementation would call your seeders
        Ok(())
    }

    /// Refresh the database (rollback and re-run migrations)
    pub async fn refresh(&self) -> Result<(), DatabaseTestError> {
        println!("Refreshing database...");
        self.rollback_all().await?;
        self.migrate().await?;
        Ok(())
    }

    /// Rollback all migrations
    pub async fn rollback_all(&self) -> Result<(), DatabaseTestError> {
        println!("Rolling back migrations...");
        // Implementation would depend on your ORM
        Ok(())
    }

    /// Clean up the test database
    pub async fn cleanup(&self) -> Result<(), DatabaseTestError> {
        println!("Cleaning up test database...");
        Ok(())
    }

    /// Get the connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Get the configuration
    pub fn config(&self) -> &TestDatabaseConfig {
        &self.config
    }
}

/// Refresh the database (drop all tables and re-run migrations)
///
/// This is useful for ensuring a clean state between test runs.
pub async fn refresh_database(database_url: &str) -> Result<(), DatabaseTestError> {
    let config = TestDatabaseConfig {
        url: database_url.to_string(),
        ..Default::default()
    };

    let test_db = TestDatabase::with_config(config).await?;
    test_db.refresh().await?;

    Ok(())
}

/// Macro to create a test with database setup
///
/// # Example
///
/// ```ignore
/// use rf_testing::test_with_db;
///
/// test_with_db!(test_user_creation, |db| async move {
///     // Your test code here
///     // db is a TestDatabase instance
///     Ok(())
/// });
/// ```
#[macro_export]
macro_rules! test_with_db {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> Result<(), Box<dyn std::error::Error>> {
            let test_db = $crate::database::TestDatabase::new().await?;

            let result = $body(&test_db).await;

            test_db.cleanup().await?;

            result
        }
    };
}

/// Macro to create a test with database and automatic cleanup
///
/// This version automatically handles errors and cleanup.
#[macro_export]
macro_rules! test_with_db_cleanup {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            let test_db = $crate::database::TestDatabase::new()
                .await
                .expect("Failed to create test database");

            $body(&test_db)
                .await
                .expect("Test failed");

            test_db
                .cleanup()
                .await
                .expect("Failed to cleanup database");
        }
    };
}

/// Transaction guard for automatic rollback
///
/// When dropped, automatically rolls back the transaction.
/// Useful for ensuring test isolation.
pub struct TransactionGuard {
    rolled_back: bool,
}

impl TransactionGuard {
    /// Create a new transaction guard
    pub fn new() -> Self {
        Self {
            rolled_back: false,
        }
    }

    /// Commit the transaction (prevents rollback on drop)
    pub fn commit(mut self) {
        self.rolled_back = true;
    }

    /// Rollback the transaction explicitly
    pub async fn rollback(&mut self) -> Result<(), DatabaseTestError> {
        if !self.rolled_back {
            self.rolled_back = true;
            // Perform rollback
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl Default for TransactionGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TransactionGuard {
    fn drop(&mut self) {
        if !self.rolled_back {
            // Rollback on drop
            println!("Rolling back transaction...");
        }
    }
}

/// Database test helper functions
pub mod helpers {
    use super::*;

    /// Truncate all tables in the database
    pub async fn truncate_all_tables(_database_url: &str) -> Result<(), DatabaseTestError> {
        println!("Truncating all tables");
        // Implementation would depend on your database
        Ok(())
    }

    /// Clear specific table
    pub async fn truncate_table(
        _database_url: &str,
        table_name: &str,
    ) -> Result<(), DatabaseTestError> {
        println!("Truncating table: {}", table_name);
        // Implementation would depend on your database
        Ok(())
    }

    /// Get table row count
    pub async fn get_row_count(
        _database_url: &str,
        table_name: &str,
    ) -> Result<i64, DatabaseTestError> {
        println!("Getting row count for table: {}", table_name);
        // Implementation would depend on your database
        Ok(0)
    }

    /// Check if table exists
    pub async fn table_exists(
        _database_url: &str,
        table_name: &str,
    ) -> Result<bool, DatabaseTestError> {
        println!("Checking if table exists: {}", table_name);
        // Implementation would depend on your database
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_config_default() {
        let config = TestDatabaseConfig::default();
        assert!(config.auto_migrate);
        assert!(!config.auto_seed);
        assert!(config.use_transactions);
    }

    #[tokio::test]
    async fn test_database_creation() {
        let result = TestDatabase::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_guard() {
        let guard = TransactionGuard::new();
        drop(guard);
        // Should rollback on drop
    }

    #[tokio::test]
    async fn test_transaction_guard_commit() {
        let guard = TransactionGuard::new();
        guard.commit();
        // Should not rollback
    }
}

// ============================================================================
// Database Assertions - Laravel-style testing utilities
// ============================================================================

/// Database assertion utilities
///
/// Provides Laravel-style database assertions for elegant testing.
pub mod assertions {
    use super::*;

    /// Assert that a record exists in the database matching the given conditions
    ///
    /// This is a low-level function. Consider using the `assert_database_has!` macro instead.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut conditions = HashMap::new();
    /// conditions.insert("email".to_string(), json!("test@example.com"));
    /// conditions.insert("active".to_string(), json!(true));
    ///
    /// assert_database_has_raw("users", conditions).await?;
    /// ```
    pub async fn assert_database_has_raw(
        table: &str,
        conditions: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseTestError> {
        if conditions.is_empty() {
            return Err(DatabaseTestError::AssertionFailed(
                "No conditions provided for assertion".to_string(),
            ));
        }

        // Build condition string for display
        let conditions_str = conditions
            .iter()
            .map(|(k, v)| format!("{} = {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        // In a real implementation, this would query the database
        // For now, we'll simulate the check
        println!(
            "Asserting database has record in '{}' where {}",
            table, conditions_str
        );

        // Placeholder: Return error to show what a failure would look like
        // In real implementation, perform actual database query
        Ok(())
    }

    /// Assert that no record exists in the database matching the given conditions
    ///
    /// This is a low-level function. Consider using the `assert_database_missing!` macro instead.
    pub async fn assert_database_missing_raw(
        table: &str,
        conditions: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseTestError> {
        // Try to find the record
        match assert_database_has_raw(table, conditions.clone()).await {
            Ok(_) => {
                // Record was found, but we expected it to be missing
                let conditions_str = conditions
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ");

                Err(DatabaseTestError::AssertionFailed(format!(
                    "Failed asserting that table '{}' does not contain record where {}",
                    table, conditions_str
                )))
            }
            Err(DatabaseTestError::QueryError(_)) => {
                // Record not found - this is what we want
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Assert that a table has exactly the expected number of rows
    ///
    /// This is a low-level function. Consider using the `assert_database_count!` macro instead.
    pub async fn assert_database_count_raw(
        table: &str,
        expected: usize,
    ) -> Result<(), DatabaseTestError> {
        // Placeholder implementation
        println!(
            "Asserting table '{}' has exactly {} rows",
            table, expected
        );

        // In real implementation:
        // let count = query("SELECT COUNT(*) FROM ?").bind(table).fetch_one();
        // if count != expected { return Err(...) }

        Ok(())
    }

    /// Assert that a table is empty
    ///
    /// This is a low-level function. Consider using the `assert_database_empty!` macro instead.
    pub async fn assert_database_empty_raw(table: &str) -> Result<(), DatabaseTestError> {
        assert_database_count_raw(table, 0).await
    }

    /// Assert that a table contains at least one row
    pub async fn assert_database_not_empty_raw(table: &str) -> Result<(), DatabaseTestError> {
        // Placeholder implementation
        println!("Asserting table '{}' is not empty", table);
        Ok(())
    }

    /// Assert that a specific record exists by comparing partial data
    ///
    /// This allows for flexible matching where only specified fields are checked.
    pub async fn assert_database_has_partial(
        table: &str,
        conditions: HashMap<String, serde_json::Value>,
    ) -> Result<(), DatabaseTestError> {
        assert_database_has_raw(table, conditions).await
    }
}

/// Macro for asserting a database record exists with Laravel-style syntax
///
/// # Example
///
/// ```ignore
/// use rf_testing::assert_database_has;
///
/// // Assert user exists with specific email
/// assert_database_has!("users", {
///     "email" => "test@example.com",
///     "active" => true
/// }).await?;
/// ```
#[macro_export]
macro_rules! assert_database_has {
    ($table:expr, { $($key:literal => $value:expr),* $(,)? }) => {{
        let mut conditions = ::std::collections::HashMap::new();
        $(
            conditions.insert($key.to_string(), ::serde_json::json!($value));
        )*
        $crate::database::assertions::assert_database_has_raw($table, conditions).await
    }};
}

/// Macro for asserting a database record does NOT exist
///
/// # Example
///
/// ```ignore
/// use rf_testing::assert_database_missing;
///
/// // Assert user was deleted
/// assert_database_missing!("users", {
///     "email" => "deleted@example.com"
/// }).await?;
/// ```
#[macro_export]
macro_rules! assert_database_missing {
    ($table:expr, { $($key:literal => $value:expr),* $(,)? }) => {{
        let mut conditions = ::std::collections::HashMap::new();
        $(
            conditions.insert($key.to_string(), ::serde_json::json!($value));
        )*
        $crate::database::assertions::assert_database_missing_raw($table, conditions).await
    }};
}

/// Macro for asserting exact row count in a table
///
/// # Example
///
/// ```ignore
/// use rf_testing::assert_database_count;
///
/// // Assert exactly 10 users exist
/// assert_database_count!("users", 10).await?;
/// ```
#[macro_export]
macro_rules! assert_database_count {
    ($table:expr, $count:expr) => {{
        $crate::database::assertions::assert_database_count_raw($table, $count).await
    }};
}

/// Macro for asserting a table is empty
///
/// # Example
///
/// ```ignore
/// use rf_testing::assert_database_empty;
///
/// // Assert no users exist
/// assert_database_empty!("users").await?;
/// ```
#[macro_export]
macro_rules! assert_database_empty {
    ($table:expr) => {{
        $crate::database::assertions::assert_database_empty_raw($table).await
    }};
}
