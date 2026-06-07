//! # Migration System with Tracking
//!
//! Laravel-inspired database migration system with full tracking capabilities.
//! Supports up/down migrations, batch rollback, and migration status tracking.
//!
//! ## Features
//!
//! - Migration trait for defining up/down operations
//! - Automatic migration tracking in database
//! - Batch-based rollback support
//! - Fresh migrations (drop all and re-run)
//! - Migration status reporting
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::migrations::*;
//! use rf_orm::schema_builder::Blueprint;
//! use async_trait::async_trait;
//! use sea_orm::DatabaseConnection;
//!
//! pub struct CreatePostsTable;
//!
//! #[async_trait]
//! impl Migration for CreatePostsTable {
//!     fn name(&self) -> &str {
//!         "2024_01_01_000001_create_posts_table"
//!     }
//!
//!     async fn up(&self, schema: &SchemaContext) -> Result<(), MigrationError> {
//!         schema.create("posts", |table: &mut Blueprint| {
//!             table.id();
//!             table.string("title");
//!             table.text("body");
//!             table.timestamps();
//!         }).await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
//!         Ok(())
//!     }
//!
//!     async fn down(&self, schema: &SchemaContext) -> Result<(), MigrationError> {
//!         schema.drop("posts").await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
//!         Ok(())
//!     }
//! }
//!
//! # async fn example(db: DatabaseConnection) -> Result<(), MigrationError> {
//! // Usage
//! let mut migrator = Migrator::new(db);
//! migrator.add_migration(Box::new(CreatePostsTable));
//!
//! // Run migrations
//! let result = migrator.run().await?;
//! println!("Ran {} migrations in batch {}", result.migrations_run, result.batch);
//!
//! // Rollback
//! migrator.rollback(None).await?; // Last batch
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

use crate::schema_builder::Schema;

/// Schema context for migrations
///
/// Provides access to schema operations within migrations
pub struct SchemaContext {
    schema: Schema,
}

impl SchemaContext {
    /// Create a new schema context
    ///
    /// Sets up schema with the given database connection
    pub async fn new(db: Arc<DatabaseConnection>) -> Self {
        // Set global connection for any static usage
        Schema::set_connection(Arc::clone(&db)).await;
        // Create instance for direct usage
        Self {
            schema: Schema::new(db),
        }
    }

    /// Create a table
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::migrations::*;
    /// # use rf_orm::schema_builder::Blueprint;
    /// # async fn example(schema: &SchemaContext) -> Result<(), Box<dyn std::error::Error>> {
    /// schema.create("posts", |table: &mut Blueprint| {
    ///     table.id();
    ///     table.string("title");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create<F>(&self, table_name: &str, callback: F) -> Result<(), crate::DbError>
    where
        F: FnOnce(&mut crate::schema_builder::Blueprint),
    {
        self.schema.create(table_name, callback).await
    }

    /// Modify an existing table
    pub async fn table<F>(&self, table_name: &str, callback: F) -> Result<(), crate::DbError>
    where
        F: FnOnce(&mut crate::schema_builder::Blueprint),
    {
        self.schema.table(table_name, callback).await
    }

    /// Drop a table
    pub async fn drop(&self, table_name: &str) -> Result<(), crate::DbError> {
        self.schema.drop(table_name).await
    }

    /// Drop a table if it exists
    pub async fn drop_if_exists(&self, table_name: &str) -> Result<(), crate::DbError> {
        self.schema.drop_if_exists(table_name).await
    }
}

/// Migration error types
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Database error during migration
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    /// Migration already applied
    #[error("Migration '{0}' has already been applied")]
    AlreadyApplied(String),

    /// Migration not found
    #[error("Migration '{0}' not found")]
    NotFound(String),

    /// Migration failed during execution
    #[error("Migration '{migration}' failed: {error}")]
    ExecutionFailed { migration: String, error: String },

    /// No migrations to rollback
    #[error("No migrations to rollback")]
    NoMigrationsToRollback,

    /// Invalid migration state
    #[error("Invalid migration state: {0}")]
    InvalidState(String),

    /// Schema error
    #[error("Schema error: {0}")]
    SchemaError(String),
}

/// Result type for migration operations
pub type MigrationResult<T> = Result<T, MigrationError>;

/// Migration trait that all migrations must implement
///
/// Defines the interface for database migrations with up and down operations.
#[async_trait]
pub trait Migration: Send + Sync {
    /// Returns the unique name of the migration
    ///
    /// Convention: `YYYY_MM_DD_HHMMSS_description`
    /// Example: `2024_01_01_000001_create_posts_table`
    fn name(&self) -> &str;

    /// Execute the migration (forward operation)
    ///
    /// # Arguments
    ///
    /// * `schema` - Schema context for creating/modifying tables
    async fn up(&self, schema: &SchemaContext) -> MigrationResult<()>;

    /// Rollback the migration (reverse operation)
    ///
    /// # Arguments
    ///
    /// * `schema` - Schema context for dropping/modifying tables
    async fn down(&self, schema: &SchemaContext) -> MigrationResult<()>;
}

/// Result of migration operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Number of migrations that were run
    pub migrations_run: usize,

    /// The batch number
    pub batch: usize,

    /// List of successful migrations
    pub successful: Vec<String>,

    /// List of failed migrations with error messages
    pub failed: Vec<(String, String)>,
}

impl BatchResult {
    /// Create a new empty result
    pub fn new(batch: usize) -> Self {
        Self {
            migrations_run: 0,
            batch,
            successful: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Check if all migrations succeeded
    pub fn is_successful(&self) -> bool {
        self.failed.is_empty()
    }

    /// Add a successful migration
    pub fn add_success(&mut self, name: String) {
        self.successful.push(name);
        self.migrations_run += 1;
    }

    /// Add a failed migration
    pub fn add_failure(&mut self, name: String, error: String) {
        self.failed.push((name, error));
    }
}

impl fmt::Display for BatchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Batch {}: {} migrations run, {} successful, {} failed",
            self.batch,
            self.migrations_run,
            self.successful.len(),
            self.failed.len()
        )
    }
}

/// Status of a single migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStatus {
    /// Migration name
    pub name: String,

    /// Whether the migration has been executed
    pub executed: bool,

    /// Batch number (if executed)
    pub batch: Option<usize>,

    /// Execution timestamp (if executed)
    pub executed_at: Option<DateTime<Utc>>,
}

impl fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.executed {
            write!(
                f,
                "[X] {} (batch: {}, executed: {})",
                self.name,
                self.batch.unwrap_or(0),
                self.executed_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            write!(f, "[ ] {} (pending)", self.name)
        }
    }
}

/// Migration tracking record
#[derive(Debug, Clone)]
struct MigrationRecord {
    #[allow(dead_code)]
    pub id: i64,
    pub migration: String,
    pub batch: i64,
    pub executed_at: DateTime<Utc>,
}

/// Migration manager
///
/// Manages the execution, tracking, and rollback of database migrations.
pub struct Migrator {
    db: Arc<DatabaseConnection>,
    migrations: Vec<Box<dyn Migration>>,
}

impl Migrator {
    /// Create a new migrator instance
    ///
    /// # Arguments
    ///
    /// * `db` - Database connection
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db: Arc::new(db),
            migrations: Vec::new(),
        }
    }

    /// Add a migration to the migrator
    ///
    /// # Arguments
    ///
    /// * `migration` - Boxed migration implementation
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }

    /// Add multiple migrations at once
    ///
    /// # Arguments
    ///
    /// * `migrations` - Vector of boxed migrations
    pub fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration>>) {
        self.migrations.extend(migrations);
    }

    /// Ensure the migrations table exists
    async fn ensure_migrations_table(&self) -> MigrationResult<()> {
        let backend = self.db.get_database_backend();

        let create_table_sql = match backend {
            DbBackend::Postgres => {
                r#"
                CREATE TABLE IF NOT EXISTS migrations (
                    id SERIAL PRIMARY KEY,
                    migration VARCHAR(255) NOT NULL UNIQUE,
                    batch INTEGER NOT NULL,
                    executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
                "#
            }
            DbBackend::MySql => {
                r#"
                CREATE TABLE IF NOT EXISTS migrations (
                    id INTEGER PRIMARY KEY AUTO_INCREMENT,
                    migration VARCHAR(255) NOT NULL UNIQUE,
                    batch INTEGER NOT NULL,
                    executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
                "#
            }
            DbBackend::Sqlite => {
                r#"
                CREATE TABLE IF NOT EXISTS migrations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    migration TEXT NOT NULL UNIQUE,
                    batch INTEGER NOT NULL,
                    executed_at TEXT DEFAULT CURRENT_TIMESTAMP
                )
                "#
            }
        };

        self.db
            .execute(Statement::from_string(backend, create_table_sql))
            .await?;

        Ok(())
    }

    /// Get all executed migrations from the database
    async fn get_executed_migrations(&self) -> MigrationResult<Vec<MigrationRecord>> {
        let backend = self.db.get_database_backend();
        let query = Statement::from_string(
            backend,
            "SELECT id, migration, batch, executed_at FROM migrations ORDER BY id",
        );

        let results = self.db.query_all(query).await?;

        let mut records = Vec::new();
        for row in results {
            let id: i64 = row.try_get("", "id")?;
            let migration: String = row.try_get("", "migration")?;
            let batch: i64 = row.try_get("", "batch")?;

            // Handle different timestamp formats across databases
            let executed_at = if let Ok(dt) = row.try_get::<DateTime<Utc>>("", "executed_at") {
                dt
            } else if let Ok(s) = row.try_get::<String>("", "executed_at") {
                DateTime::parse_from_rfc3339(&s)
                    .or_else(|_| DateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S"))
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now())
            } else {
                Utc::now()
            };

            records.push(MigrationRecord {
                id,
                migration,
                batch,
                executed_at,
            });
        }

        Ok(records)
    }

    /// Get the next batch number
    async fn get_next_batch(&self) -> MigrationResult<usize> {
        let backend = self.db.get_database_backend();
        let query =
            Statement::from_string(backend, "SELECT MAX(batch) as max_batch FROM migrations");

        let result = self.db.query_one(query).await?;

        if let Some(row) = result {
            let max_batch: Option<i64> = row.try_get("", "max_batch").ok();
            Ok(max_batch.map(|b| b as usize + 1).unwrap_or(1))
        } else {
            Ok(1)
        }
    }

    /// Record a migration execution
    async fn record_migration(&self, name: &str, batch: usize) -> MigrationResult<()> {
        let backend = self.db.get_database_backend();

        let insert_sql = match backend {
            DbBackend::Postgres | DbBackend::MySql => {
                format!(
                    "INSERT INTO migrations (migration, batch) VALUES ('{}', {})",
                    name, batch
                )
            }
            DbBackend::Sqlite => {
                format!(
                    "INSERT INTO migrations (migration, batch, executed_at) VALUES ('{}', {}, datetime('now'))",
                    name, batch
                )
            }
        };

        self.db
            .execute(Statement::from_string(backend, insert_sql))
            .await?;

        Ok(())
    }

    /// Remove a migration record
    async fn remove_migration(&self, name: &str) -> MigrationResult<()> {
        let backend = self.db.get_database_backend();
        let escaped_name = name.replace('\'', "''");
        let delete_sql = format!("DELETE FROM migrations WHERE migration = '{}'", escaped_name);

        self.db
            .execute(Statement::from_string(backend, delete_sql))
            .await?;

        Ok(())
    }

    /// Run all pending migrations
    ///
    /// Executes all migrations that haven't been run yet in a single batch.
    ///
    /// # Returns
    ///
    /// `BatchResult` containing the results of the migration run
    pub async fn run(&self) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;

        let executed = self.get_executed_migrations().await?;
        let executed_names: Vec<String> = executed.iter().map(|r| r.migration.clone()).collect();

        let batch = self.get_next_batch().await?;
        let mut result = BatchResult::new(batch);

        for migration in &self.migrations {
            let name = migration.name();

            if executed_names.contains(&name.to_string()) {
                continue; // Skip already executed migrations
            }

            let schema_ctx = SchemaContext::new(self.db.clone()).await;
            match migration.up(&schema_ctx).await {
                Ok(_) => match self.record_migration(name, batch).await {
                    Ok(_) => {
                        result.add_success(name.to_string());
                        tracing::info!("Migration '{}' executed successfully", name);
                    }
                    Err(e) => {
                        let error = format!("Failed to record migration: {}", e);
                        result.add_failure(name.to_string(), error.clone());
                        tracing::error!("Failed to record migration '{}': {}", name, e);
                    }
                },
                Err(e) => {
                    let error = e.to_string();
                    result.add_failure(name.to_string(), error.clone());
                    tracing::error!("Migration '{}' failed: {}", name, error);
                    break; // Stop on first error
                }
            }
        }

        Ok(result)
    }

    /// Rollback migrations
    ///
    /// Rolls back the last N batches of migrations.
    ///
    /// # Arguments
    ///
    /// * `steps` - Number of batches to rollback (None = rollback last batch)
    pub async fn rollback(&self, steps: Option<usize>) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;

        let executed = self.get_executed_migrations().await?;

        if executed.is_empty() {
            return Err(MigrationError::NoMigrationsToRollback);
        }

        // Determine which batches to rollback
        let max_batch = executed.iter().map(|r| r.batch).max().unwrap_or(0);
        let steps = steps.unwrap_or(1);
        let min_batch = (max_batch - steps as i64 + 1).max(1);

        let to_rollback: Vec<_> = executed
            .into_iter()
            .filter(|r| r.batch >= min_batch)
            .collect();

        let mut result = BatchResult::new(max_batch as usize);

        // Rollback in reverse order
        for record in to_rollback.iter().rev() {
            let migration = self
                .migrations
                .iter()
                .find(|m| m.name() == record.migration)
                .ok_or_else(|| MigrationError::NotFound(record.migration.clone()))?;

            let schema_ctx = SchemaContext::new(self.db.clone()).await;
            match migration.down(&schema_ctx).await {
                Ok(_) => match self.remove_migration(&record.migration).await {
                    Ok(_) => {
                        result.add_success(record.migration.clone());
                        tracing::info!("Migration '{}' rolled back successfully", record.migration);
                    }
                    Err(e) => {
                        let error = format!("Failed to remove migration record: {}", e);
                        result.add_failure(record.migration.clone(), error.clone());
                        tracing::error!(
                            "Failed to remove migration record '{}': {}",
                            record.migration,
                            e
                        );
                    }
                },
                Err(e) => {
                    let error = e.to_string();
                    result.add_failure(record.migration.clone(), error.clone());
                    tracing::error!(
                        "Migration '{}' rollback failed: {}",
                        record.migration,
                        error
                    );
                    break; // Stop on first error
                }
            }
        }

        Ok(result)
    }

    /// Rollback a specific batch
    ///
    /// # Arguments
    ///
    /// * `batch` - The batch number to rollback
    pub async fn rollback_batch(&self, batch: usize) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;

        let executed = self.get_executed_migrations().await?;

        let to_rollback: Vec<_> = executed
            .into_iter()
            .filter(|r| r.batch == batch as i64)
            .collect();

        if to_rollback.is_empty() {
            return Err(MigrationError::InvalidState(format!(
                "No migrations found in batch {}",
                batch
            )));
        }

        let mut result = BatchResult::new(batch);

        // Rollback in reverse order
        for record in to_rollback.iter().rev() {
            let migration = self
                .migrations
                .iter()
                .find(|m| m.name() == record.migration)
                .ok_or_else(|| MigrationError::NotFound(record.migration.clone()))?;

            let schema_ctx = SchemaContext::new(self.db.clone()).await;
            match migration.down(&schema_ctx).await {
                Ok(_) => match self.remove_migration(&record.migration).await {
                    Ok(_) => {
                        result.add_success(record.migration.clone());
                        tracing::info!("Migration '{}' rolled back successfully", record.migration);
                    }
                    Err(e) => {
                        let error = format!("Failed to remove migration record: {}", e);
                        result.add_failure(record.migration.clone(), error);
                    }
                },
                Err(e) => {
                    let error = e.to_string();
                    result.add_failure(record.migration.clone(), error);
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Fresh migration - rollback all and re-run
    ///
    /// Rolls back all migrations and then re-runs them from scratch.
    /// Useful for development environments.
    pub async fn fresh(&self) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;

        let executed = self.get_executed_migrations().await?;

        // Rollback all migrations
        if !executed.is_empty() {
            let _rollback_result = BatchResult::new(0);

            for record in executed.iter().rev() {
                let migration = self
                    .migrations
                    .iter()
                    .find(|m| m.name() == record.migration)
                    .ok_or_else(|| MigrationError::NotFound(record.migration.clone()))?;

                let schema_ctx = SchemaContext::new(self.db.clone()).await;
                if let Err(e) = migration.down(&schema_ctx).await {
                    tracing::warn!("Failed to rollback migration '{}': {}", record.migration, e);
                    // Continue anyway
                }

                let _ = self.remove_migration(&record.migration).await;
            }
        }

        // Re-run all migrations
        self.run().await
    }

    /// Get migration status
    ///
    /// Returns the status of all migrations (executed and pending).
    pub async fn status(&self) -> MigrationResult<Vec<MigrationStatus>> {
        self.ensure_migrations_table().await?;

        let executed = self.get_executed_migrations().await?;

        let mut statuses = Vec::new();

        for migration in &self.migrations {
            let name = migration.name().to_string();

            if let Some(record) = executed.iter().find(|r| r.migration == name) {
                statuses.push(MigrationStatus {
                    name,
                    executed: true,
                    batch: Some(record.batch as usize),
                    executed_at: Some(record.executed_at),
                });
            } else {
                statuses.push(MigrationStatus {
                    name,
                    executed: false,
                    batch: None,
                    executed_at: None,
                });
            }
        }

        Ok(statuses)
    }

    /// Get database connection
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    struct TestMigration1;

    #[async_trait]
    impl Migration for TestMigration1 {
        fn name(&self) -> &str {
            "2024_01_01_000001_test_migration_1"
        }

        async fn up(&self, _schema: &SchemaContext) -> MigrationResult<()> {
            Ok(())
        }

        async fn down(&self, _schema: &SchemaContext) -> MigrationResult<()> {
            Ok(())
        }
    }

    struct TestMigration2;

    #[async_trait]
    impl Migration for TestMigration2 {
        fn name(&self) -> &str {
            "2024_01_01_000002_test_migration_2"
        }

        async fn up(&self, _schema: &SchemaContext) -> MigrationResult<()> {
            Ok(())
        }

        async fn down(&self, _schema: &SchemaContext) -> MigrationResult<()> {
            Ok(())
        }
    }

    struct CreateTestPostsTable;

    #[async_trait]
    impl Migration for CreateTestPostsTable {
        fn name(&self) -> &str {
            "2024_01_01_000003_create_test_posts_table"
        }

        async fn up(&self, schema: &SchemaContext) -> MigrationResult<()> {
            schema
                .create("test_posts", |table| {
                    table.id();
                    table.string("title");
                    table.text("body");
                    table.boolean("published").default("false");
                    table.timestamps();
                })
                .await
                .map_err(|e| MigrationError::SchemaError(e.to_string()))?;
            Ok(())
        }

        async fn down(&self, schema: &SchemaContext) -> MigrationResult<()> {
            schema
                .drop("test_posts")
                .await
                .map_err(|e| MigrationError::SchemaError(e.to_string()))?;
            Ok(())
        }
    }

    async fn setup_test_db() -> DatabaseConnection {
        Database::connect("sqlite::memory:")
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_migrator_creation() {
        let db = setup_test_db().await;
        let migrator = Migrator::new(db);
        assert_eq!(migrator.migrations.len(), 0);
    }

    #[tokio::test]
    async fn test_add_migration() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));
        assert_eq!(migrator.migrations.len(), 1);

        migrator.add_migration(Box::new(TestMigration2));
        assert_eq!(migrator.migrations.len(), 2);
    }

    #[tokio::test]
    async fn test_add_multiple_migrations() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        let migrations: Vec<Box<dyn Migration>> =
            vec![Box::new(TestMigration1), Box::new(TestMigration2)];

        migrator.add_migrations(migrations);
        assert_eq!(migrator.migrations.len(), 2);
    }

    #[tokio::test]
    async fn test_ensure_migrations_table() {
        let db = setup_test_db().await;
        let migrator = Migrator::new(db);

        // Should not fail
        migrator.ensure_migrations_table().await.unwrap();

        // Should be idempotent
        migrator.ensure_migrations_table().await.unwrap();
    }

    #[tokio::test]
    async fn test_migration_tracking() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));
        migrator.add_migration(Box::new(TestMigration2));

        // Run migrations
        let result = migrator.run().await.unwrap();
        assert_eq!(result.migrations_run, 2);
        assert_eq!(result.batch, 1);
        assert!(result.is_successful());

        // Check status
        let status = migrator.status().await.unwrap();
        assert_eq!(status.len(), 2);
        assert!(status[0].executed);
        assert!(status[1].executed);
        assert_eq!(status[0].batch, Some(1));
        assert_eq!(status[1].batch, Some(1));

        // Running again should not execute anything
        let result2 = migrator.run().await.unwrap();
        assert_eq!(result2.migrations_run, 0);
    }

    #[tokio::test]
    async fn test_rollback() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));
        migrator.add_migration(Box::new(TestMigration2));

        // Run migrations
        migrator.run().await.unwrap();

        // Rollback last batch
        let result = migrator.rollback(None).await.unwrap();
        assert_eq!(result.migrations_run, 2);

        // Check status
        let status = migrator.status().await.unwrap();
        assert!(!status[0].executed);
        assert!(!status[1].executed);
    }

    #[tokio::test]
    async fn test_rollback_steps() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));
        migrator.add_migration(Box::new(TestMigration2));

        // Run migrations - batch 1
        migrator.run().await.unwrap();

        // Add and run more migrations - batch 2
        migrator.add_migration(Box::new(CreateTestPostsTable));
        migrator.run().await.unwrap();

        // Rollback 1 step (only batch 2)
        let result = migrator.rollback(Some(1)).await.unwrap();
        assert!(result.migrations_run <= 1);

        // First 2 migrations should still be executed
        let status = migrator.status().await.unwrap();
        assert!(status[0].executed);
        assert!(status[1].executed);
    }

    #[tokio::test]
    async fn test_rollback_batch() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));

        // Run migrations
        migrator.run().await.unwrap();

        // Rollback specific batch
        let result = migrator.rollback_batch(1).await.unwrap();
        assert_eq!(result.migrations_run, 1);

        // Check status
        let status = migrator.status().await.unwrap();
        assert!(!status[0].executed);
    }

    #[tokio::test]
    async fn test_fresh_migrations() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(TestMigration1));
        migrator.add_migration(Box::new(TestMigration2));

        // Run migrations
        migrator.run().await.unwrap();

        // Fresh - should rollback and re-run
        let result = migrator.fresh().await.unwrap();
        assert_eq!(result.migrations_run, 2);
        assert_eq!(result.batch, 1);

        // All migrations should be executed
        let status = migrator.status().await.unwrap();
        assert!(status[0].executed);
        assert!(status[1].executed);
    }

    #[tokio::test]
    async fn test_migration_with_schema_operations() {
        let db = setup_test_db().await;
        let mut migrator = Migrator::new(db);

        migrator.add_migration(Box::new(CreateTestPostsTable));

        // Run migration
        let result = migrator.run().await.unwrap();
        assert_eq!(result.migrations_run, 1);
        assert!(result.is_successful());

        // Rollback
        let rollback_result = migrator.rollback(None).await.unwrap();
        assert_eq!(rollback_result.migrations_run, 1);
        assert!(rollback_result.is_successful());
    }

    #[tokio::test]
    async fn test_batch_result_display() {
        let mut result = BatchResult::new(1);
        result.add_success("migration_1".to_string());
        result.add_success("migration_2".to_string());

        let display = result.to_string();
        assert!(display.contains("Batch 1"));
        assert!(display.contains("2 migrations run"));
        assert!(display.contains("2 successful"));
        assert!(display.contains("0 failed"));
    }

    #[tokio::test]
    async fn test_batch_result_with_failures() {
        let mut result = BatchResult::new(1);
        result.add_success("migration_1".to_string());
        result.add_failure("migration_2".to_string(), "Test error".to_string());

        assert!(!result.is_successful());
        assert_eq!(result.successful.len(), 1);
        assert_eq!(result.failed.len(), 1);
    }

    #[tokio::test]
    async fn test_migration_status_display() {
        let status = MigrationStatus {
            name: "test_migration".to_string(),
            executed: true,
            batch: Some(1),
            executed_at: Some(Utc::now()),
        };

        let display = status.to_string();
        assert!(display.contains("[X]"));
        assert!(display.contains("test_migration"));
        assert!(display.contains("batch: 1"));
    }

    #[tokio::test]
    async fn test_migration_status_pending() {
        let status = MigrationStatus {
            name: "pending_migration".to_string(),
            executed: false,
            batch: None,
            executed_at: None,
        };

        let display = status.to_string();
        assert!(display.contains("[ ]"));
        assert!(display.contains("pending_migration"));
        assert!(display.contains("pending"));
    }

    #[tokio::test]
    async fn test_no_migrations_to_rollback() {
        let db = setup_test_db().await;
        let migrator = Migrator::new(db);

        // Try to rollback without any migrations
        let result = migrator.rollback(None).await;
        assert!(result.is_err());
        match result {
            Err(MigrationError::NoMigrationsToRollback) => (),
            _ => panic!("Expected NoMigrationsToRollback error"),
        }
    }

    #[tokio::test]
    async fn test_migration_error_types() {
        // Test error display
        let err = MigrationError::AlreadyApplied("test_migration".to_string());
        assert!(err.to_string().contains("already been applied"));

        let err = MigrationError::NotFound("missing_migration".to_string());
        assert!(err.to_string().contains("not found"));

        let err = MigrationError::InvalidState("bad state".to_string());
        assert!(err.to_string().contains("Invalid migration state"));
    }
}
