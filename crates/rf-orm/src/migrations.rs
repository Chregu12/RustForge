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
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;

use crate::{schema_builder::Schema, DatabaseManager};

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

enum RegisteredMigration {
    Rust(Box<dyn Migration>),
    Sql {
        name: String,
        up: String,
        down: String,
    },
}

impl RegisteredMigration {
    fn name(&self) -> &str {
        match self {
            Self::Rust(migration) => migration.name(),
            Self::Sql { name, .. } => name,
        }
    }
}

/// Migration manager
///
/// Manages the execution, tracking, and rollback of database migrations.
pub struct Migrator {
    db: Arc<DatabaseConnection>,
    migrations: Vec<RegisteredMigration>,
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

    /// Use the same connection pool as the application's database manager.
    pub fn from_manager(manager: &DatabaseManager) -> Self {
        Self {
            db: manager.shared_connection(),
            migrations: Vec::new(),
        }
    }

    /// Add a migration to the migrator
    ///
    /// # Arguments
    ///
    /// * `migration` - Boxed migration implementation
    pub fn add_migration(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(RegisteredMigration::Rust(migration));
    }

    /// Add multiple migrations at once
    ///
    /// # Arguments
    ///
    /// * `migrations` - Vector of boxed migrations
    pub fn add_migrations(&mut self, migrations: Vec<Box<dyn Migration>>) {
        self.migrations
            .extend(migrations.into_iter().map(RegisteredMigration::Rust));
    }

    /// Register a pair of SQL scripts. Each script and its ledger change run
    /// in one transaction on SQLite or PostgreSQL. Scripts must omit explicit
    /// transaction control and use the SQL dialect of the configured database.
    pub fn add_sql_migration(&mut self, name: String, up: String, down: String) {
        self.migrations
            .push(RegisteredMigration::Sql { name, up, down });
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
        let integer_type = if backend == DbBackend::MySql {
            "SIGNED"
        } else {
            "BIGINT"
        };
        let query = Statement::from_string(backend, format!(
            "SELECT CAST(id AS {integer_type}) AS id, migration, CAST(batch AS {integer_type}) AS batch, executed_at FROM migrations ORDER BY id"
        ));

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

    async fn update_ledger(
        db: &impl ConnectionTrait,
        name: &str,
        batch: Option<usize>,
    ) -> MigrationResult<()> {
        let backend = db.get_database_backend();
        let (name_param, batch_param) = if backend == DbBackend::Postgres {
            ("$1", "$2")
        } else {
            ("?", "?")
        };
        let statement = if let Some(batch) = batch {
            let batch = i32::try_from(batch).map_err(|_| {
                MigrationError::InvalidState("Migration batch limit reached".into())
            })?;
            Statement::from_sql_and_values(
                backend,
                format!("INSERT INTO migrations (migration, batch) VALUES ({name_param}, {batch_param})"),
                [name.into(), batch.into()],
            )
        } else {
            Statement::from_sql_and_values(
                backend,
                format!("DELETE FROM migrations WHERE migration = {name_param}"),
                [name.into()],
            )
        };
        if db.execute(statement).await?.rows_affected() != 1 {
            return Err(MigrationError::InvalidState(format!(
                "Migration record for '{name}' changed concurrently"
            )));
        }
        Ok(())
    }

    async fn apply(
        &self,
        migration: &RegisteredMigration,
        batch: Option<usize>,
    ) -> MigrationResult<()> {
        match migration {
            RegisteredMigration::Rust(migration) => {
                let schema = SchemaContext::new(self.db.clone()).await;
                if batch.is_some() {
                    migration.up(&schema).await?;
                } else {
                    migration.down(&schema).await?;
                }
                Self::update_ledger(self.db.as_ref(), migration.name(), batch).await
            }
            RegisteredMigration::Sql { name, up, down } => {
                if self.db.get_database_backend() == DbBackend::MySql {
                    return Err(MigrationError::InvalidState(
                        "Transactional SQL migrations require SQLite or PostgreSQL".into(),
                    ));
                }
                let tx = self.db.begin().await?;
                // Claim the ledger entry before touching schema so competing
                // runners cannot apply or undo the same migration twice.
                Self::update_ledger(&tx, name, batch).await?;
                tx.execute_unprepared(if batch.is_some() { up } else { down })
                    .await?;
                tx.commit().await?;
                Ok(())
            }
        }
    }

    /// Run all pending migrations as a single numbered batch.
    pub async fn run(&self) -> MigrationResult<BatchResult> {
        self.run_pending(false).await
    }

    /// Run pending migrations, giving each one its own batch number.
    pub async fn run_stepwise(&self) -> MigrationResult<BatchResult> {
        self.run_pending(true).await
    }

    async fn run_pending(&self, stepwise: bool) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;
        let executed = self.get_executed_migrations().await?;
        let batch = executed
            .iter()
            .map(|record| record.batch as usize)
            .max()
            .unwrap_or(0)
            + 1;
        let mut result = BatchResult::new(batch);

        for migration in &self.migrations {
            let name = migration.name();
            if executed.iter().any(|record| record.migration == name) {
                continue;
            }
            let migration_batch = batch + if stepwise { result.migrations_run } else { 0 };
            match self.apply(migration, Some(migration_batch)).await {
                Ok(()) => result.add_success(name.to_owned()),
                Err(error) => {
                    result.add_failure(name.to_owned(), error.to_string());
                    break;
                }
            }
        }

        Ok(result)
    }

    async fn rollback_records(
        &self,
        records: Vec<MigrationRecord>,
        batch: usize,
    ) -> MigrationResult<BatchResult> {
        // Resolve every script first. A missing file must not leave an
        // otherwise valid rollback only partly completed.
        let migrations = records
            .iter()
            .rev()
            .map(|record| {
                self.migrations
                    .iter()
                    .find(|migration| migration.name() == record.migration)
                    .ok_or_else(|| MigrationError::NotFound(record.migration.clone()))
            })
            .collect::<MigrationResult<Vec<_>>>()?;
        let mut result = BatchResult::new(batch);
        for migration in migrations {
            match self.apply(migration, None).await {
                Ok(()) => result.add_success(migration.name().to_owned()),
                Err(error) => {
                    result.add_failure(migration.name().to_owned(), error.to_string());
                    break;
                }
            }
        }
        Ok(result)
    }

    /// Roll back the last N numbered batches (defaults to the last batch).
    pub async fn rollback(&self, steps: Option<usize>) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;
        let mut executed = self.get_executed_migrations().await?;
        let batch = executed
            .iter()
            .map(|record| record.batch)
            .max()
            .ok_or(MigrationError::NoMigrationsToRollback)?;
        let batches: std::collections::BTreeSet<_> =
            executed.iter().map(|record| record.batch).collect();
        let steps = steps.unwrap_or(1);
        if let Some(first) = batches.iter().rev().take(steps).next_back() {
            executed.retain(|record| record.batch >= *first);
        } else {
            executed.clear();
        }
        self.rollback_records(executed, batch as usize).await
    }

    /// Roll back a specific batch in reverse application order.
    pub async fn rollback_batch(&self, batch: usize) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;
        let executed: Vec<_> = self
            .get_executed_migrations()
            .await?
            .into_iter()
            .filter(|record| record.batch as usize == batch)
            .collect();
        if executed.is_empty() {
            return Err(MigrationError::InvalidState(format!(
                "No migrations found in batch {batch}"
            )));
        }
        self.rollback_records(executed, batch).await
    }

    /// Roll back up to `count` migrations in reverse application order.
    pub async fn rollback_latest(&self, count: usize) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;
        let mut records = self.get_executed_migrations().await?;
        let latest = records.split_off(records.len().saturating_sub(count));
        self.rollback_records(latest, 0).await
    }

    /// Roll back every recorded migration.
    pub async fn reset(&self) -> MigrationResult<BatchResult> {
        self.ensure_migrations_table().await?;
        let executed = self.get_executed_migrations().await?;
        self.rollback_records(executed, 0).await
    }

    /// Roll back every recorded migration, then reapply all registered migrations.
    pub async fn fresh(&self) -> MigrationResult<BatchResult> {
        let rollback = self.reset().await?;
        if !rollback.is_successful() {
            return Ok(rollback);
        }
        self.run().await
    }

    /// Get migration status
    ///
    /// Returns the status of all migrations (executed and pending).
    pub async fn status(&self) -> MigrationResult<Vec<MigrationStatus>> {
        let backend = self.db.get_database_backend();
        let table_exists = match backend {
            DbBackend::Sqlite => "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'migrations'",
            DbBackend::Postgres => "SELECT 1 WHERE to_regclass('migrations') IS NOT NULL",
            DbBackend::MySql => "SELECT 1 FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = 'migrations'",
        };
        let executed = if self
            .db
            .query_one(Statement::from_string(backend, table_exists))
            .await?
            .is_some()
        {
            self.get_executed_migrations().await?
        } else {
            Vec::new()
        };

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
    use crate::DatabaseConfig;
    use sea_orm::Database;

    #[tokio::test]
    async fn failed_sql_script_leaves_schema_and_ledger_consistent() {
        let db = DatabaseManager::connect(DatabaseConfig {
            max_connections: 1,
            min_connections: 1,
            ..Default::default()
        })
        .await
        .unwrap();
        let mut migrator = Migrator::from_manager(&db);
        migrator.add_sql_migration(
            "test_sql".into(),
            "CREATE TABLE posts (id INTEGER PRIMARY KEY); INVALID SQL;".into(),
            "DROP TABLE posts;".into(),
        );
        assert!(!migrator.run().await.unwrap().is_successful());
        assert!(!migrator.status().await.unwrap()[0].executed);
        assert!(db
            .connection()
            .execute_unprepared("SELECT * FROM posts")
            .await
            .is_err());

        let mut migrator = Migrator::from_manager(&db);
        migrator.add_sql_migration(
            "test_sql".into(),
            "CREATE TABLE posts (id INTEGER PRIMARY KEY); INSERT INTO posts VALUES (1);".into(),
            "DROP TABLE posts; INVALID SQL;".into(),
        );
        assert_eq!(migrator.run().await.unwrap().migrations_run, 1);
        assert!(!migrator.rollback(None).await.unwrap().is_successful());
        assert!(migrator.status().await.unwrap()[0].executed);
        assert!(db
            .connection()
            .execute_unprepared("SELECT * FROM posts")
            .await
            .is_ok());
    }

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
