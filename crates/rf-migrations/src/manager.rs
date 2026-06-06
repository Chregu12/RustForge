//! [`MigrationManager`] — the central coordinator for all migration operations.

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tracing::{info, warn};

use crate::{
    error::{MigrationError, Result},
    migration::Migration,
    record::RunResult,
    status::MigrationStatus,
};

// ---------------------------------------------------------------------------
// Internal row type
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct MigrationRow {
    pub migration: String,
    pub batch: i64,
    pub ran_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// MigrationManager
// ---------------------------------------------------------------------------

/// The central migration coordinator.
///
/// Register migrations with [`register`](Self::register) (or
/// [`register_many`](Self::register_many)), then call the lifecycle methods.
///
/// # Example
///
/// ```rust,no_run
/// use rf_migrations::{Migration, MigrationManager};
/// use async_trait::async_trait;
/// use sea_orm::DatabaseConnection;
///
/// struct AddEmailToUsers;
///
/// #[async_trait]
/// impl Migration for AddEmailToUsers {
///     fn name(&self) -> &str { "2024_03_01_000001_add_email_to_users" }
///     async fn up(&self, _db: &DatabaseConnection) -> anyhow::Result<()> { Ok(()) }
///     async fn down(&self, _db: &DatabaseConnection) -> anyhow::Result<()> { Ok(()) }
/// }
///
/// # async fn run(db: sea_orm::DatabaseConnection) -> anyhow::Result<()> {
/// let mut mgr = MigrationManager::new(db);
/// mgr.register(Box::new(AddEmailToUsers));
/// mgr.run().await?;
/// # Ok(()) }
/// ```
pub struct MigrationManager {
    db: DatabaseConnection,
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationManager {
    /// Create a new manager backed by the given connection.
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            migrations: Vec::new(),
        }
    }

    /// Register a single migration.
    pub fn register(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
    }

    /// Register multiple migrations at once.
    pub fn register_many(&mut self, migrations: Vec<Box<dyn Migration>>) {
        self.migrations.extend(migrations);
    }

    /// Return a reference to the underlying database connection.
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }

    // -----------------------------------------------------------------------
    // Public lifecycle API
    // -----------------------------------------------------------------------

    /// Apply all pending (not-yet-applied) migrations in registration order.
    ///
    /// All newly applied migrations share the same *batch* number so they can
    /// be rolled back together with a single `rollback(1)`.
    ///
    /// Returns a [`RunResult`] describing what happened. If a migration fails
    /// execution stops immediately and the error is recorded in the result.
    pub async fn run(&mut self) -> Result<RunResult> {
        self.ensure_migrations_table().await?;

        let applied = self.fetch_applied().await?;
        let applied_names: Vec<&str> = applied.iter().map(|r| r.migration.as_str()).collect();

        let batch = self.next_batch().await?;
        let mut result = RunResult::new(batch);

        for m in &self.migrations {
            let name = m.name();
            if applied_names.contains(&name) {
                continue; // already applied
            }

            match m.up(&self.db).await {
                Ok(()) => {
                    self.record(name, batch).await?;
                    info!("Migrated: {}", name);
                    result.applied.push(name.to_string());
                }
                Err(e) => {
                    warn!("Migration failed: {} — {}", name, e);
                    let msg = e.to_string();
                    result.failed.push((name.to_string(), msg));
                    break; // stop on first failure
                }
            }
        }

        Ok(result)
    }

    /// Roll back the last `steps` batches.
    ///
    /// Migrations within a batch are reversed in reverse-registration order.
    ///
    /// # Errors
    ///
    /// Returns [`MigrationError::NothingToRollback`] when no migrations have
    /// been applied, or [`MigrationError::NotFound`] when the tracking table
    /// references a migration that is not registered.
    pub async fn rollback(&mut self, steps: u32) -> Result<RunResult> {
        self.ensure_migrations_table().await?;

        let applied = self.fetch_applied().await?;
        if applied.is_empty() {
            return Err(MigrationError::NothingToRollback);
        }

        let max_batch = applied.iter().map(|r| r.batch).max().unwrap_or(0);
        let min_batch = (max_batch - steps as i64 + 1).max(1);

        let to_undo: Vec<&MigrationRow> = applied
            .iter()
            .filter(|r| r.batch >= min_batch)
            .collect();

        let mut result = RunResult::new(max_batch as u32);

        // Reverse through the migrations in reverse registration order so
        // dependencies are dropped after their dependents.
        for row in to_undo.iter().rev() {
            let m = self
                .migrations
                .iter()
                .find(|m| m.name() == row.migration)
                .ok_or_else(|| MigrationError::NotFound(row.migration.clone()))?;

            match m.down(&self.db).await {
                Ok(()) => {
                    self.unrecord(&row.migration).await?;
                    info!("Rolled back: {}", row.migration);
                    result.applied.push(row.migration.clone());
                }
                Err(e) => {
                    warn!("Rollback failed: {} — {}", row.migration, e);
                    let msg = e.to_string();
                    result.failed.push((row.migration.clone(), msg));
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Roll back *all* applied migrations.
    ///
    /// Equivalent to calling `rollback` with a steps value large enough to
    /// cover every applied batch.
    pub async fn reset(&mut self) -> Result<RunResult> {
        self.ensure_migrations_table().await?;

        let applied = self.fetch_applied().await?;
        if applied.is_empty() {
            return Ok(RunResult::new(0));
        }

        let max_batch = applied.iter().map(|r| r.batch).max().unwrap_or(1) as u32;
        self.rollback(max_batch).await
    }

    /// Roll back all applied migrations and then re-apply them from scratch.
    ///
    /// Useful in development to get a clean database state without manually
    /// dropping tables.
    pub async fn fresh(&mut self) -> Result<RunResult> {
        self.reset().await?;
        self.run().await
    }

    /// Roll back the last batch and immediately re-apply those migrations.
    ///
    /// This is the equivalent of Laravel's `migrate:refresh`. Unlike `fresh`,
    /// it only touches the most recent batch rather than wiping everything.
    pub async fn refresh(&mut self) -> Result<RunResult> {
        self.rollback(1).await?;
        self.run().await
    }

    /// Return the run-status of every registered migration.
    ///
    /// Pending migrations have `applied = false`; applied migrations include
    /// their batch number and timestamp.
    pub async fn status(&self) -> Result<Vec<MigrationStatus>> {
        self.ensure_migrations_table().await?;

        let applied = self.fetch_applied().await?;

        let statuses = self
            .migrations
            .iter()
            .map(|m| {
                let name = m.name().to_string();
                if let Some(row) = applied.iter().find(|r| r.migration == name) {
                    MigrationStatus {
                        name,
                        applied: true,
                        batch: Some(row.batch as u32),
                        applied_at: Some(row.ran_at),
                    }
                } else {
                    MigrationStatus {
                        name,
                        applied: false,
                        batch: None,
                        applied_at: None,
                    }
                }
            })
            .collect();

        Ok(statuses)
    }

    // -----------------------------------------------------------------------
    // Tracking-table helpers (private)
    // -----------------------------------------------------------------------

    /// Create the `migrations` tracking table if it does not already exist.
    async fn ensure_migrations_table(&self) -> Result<()> {
        let backend = self.db.get_database_backend();
        let sql = tracking_table_ddl(backend);
        self.db
            .execute(Statement::from_string(backend, sql))
            .await?;
        Ok(())
    }

    /// Load all rows from the `migrations` table, ordered by id.
    async fn fetch_applied(&self) -> Result<Vec<MigrationRow>> {
        let backend = self.db.get_database_backend();
        let rows = self
            .db
            .query_all(Statement::from_string(
                backend,
                "SELECT migration, batch, ran_at FROM migrations ORDER BY id",
            ))
            .await?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let migration: String = row.try_get("", "migration")?;
            let batch: i64 = row.try_get("", "batch")?;

            // ran_at is stored as TEXT in SQLite and as a native timestamp elsewhere.
            let ran_at = if let Ok(dt) = row.try_get::<DateTime<Utc>>("", "ran_at") {
                dt
            } else if let Ok(s) = row.try_get::<String>("", "ran_at") {
                parse_timestamp(&s)
            } else {
                Utc::now()
            };

            records.push(MigrationRow {
                migration,
                batch,
                ran_at,
            });
        }
        Ok(records)
    }

    /// Determine the next batch number (max existing + 1, or 1 if empty).
    async fn next_batch(&self) -> Result<u32> {
        let backend = self.db.get_database_backend();
        let result = self
            .db
            .query_one(Statement::from_string(
                backend,
                "SELECT MAX(batch) AS max_batch FROM migrations",
            ))
            .await?;

        let next = match result {
            Some(row) => {
                let max: Option<i64> = row.try_get("", "max_batch").ok();
                max.map(|v| v as u32 + 1).unwrap_or(1)
            }
            None => 1,
        };
        Ok(next)
    }

    /// Insert a row into the tracking table after a successful `up`.
    async fn record(&self, name: &str, batch: u32) -> Result<()> {
        let backend = self.db.get_database_backend();
        let sql = insert_sql(backend, name, batch);
        self.db
            .execute(Statement::from_string(backend, sql))
            .await?;
        Ok(())
    }

    /// Remove a row from the tracking table after a successful `down`.
    async fn unrecord(&self, name: &str) -> Result<()> {
        let backend = self.db.get_database_backend();
        // Escape single quotes to avoid SQL injection-like issues with
        // migration names that include apostrophes.
        let escaped = name.replace('\'', "''");
        let sql = format!("DELETE FROM migrations WHERE migration = '{}'", escaped);
        self.db
            .execute(Statement::from_string(backend, sql))
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SQL helpers (free functions for readability)
// ---------------------------------------------------------------------------

fn tracking_table_ddl(backend: DbBackend) -> String {
    match backend {
        DbBackend::Postgres => {
            r#"CREATE TABLE IF NOT EXISTS migrations (
                id         SERIAL PRIMARY KEY,
                migration  VARCHAR(255) NOT NULL UNIQUE,
                batch      INTEGER NOT NULL,
                ran_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )"#
            .to_string()
        }
        DbBackend::MySql => {
            r#"CREATE TABLE IF NOT EXISTS migrations (
                id         INT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
                migration  VARCHAR(255) NOT NULL UNIQUE,
                batch      INT NOT NULL,
                ran_at     DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#
            .to_string()
        }
        DbBackend::Sqlite => {
            r#"CREATE TABLE IF NOT EXISTS migrations (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                migration  TEXT NOT NULL UNIQUE,
                batch      INTEGER NOT NULL,
                ran_at     TEXT NOT NULL DEFAULT (datetime('now'))
            )"#
            .to_string()
        }
    }
}

fn insert_sql(backend: DbBackend, name: &str, batch: u32) -> String {
    let escaped = name.replace('\'', "''");
    match backend {
        DbBackend::Sqlite => format!(
            "INSERT INTO migrations (migration, batch, ran_at) VALUES ('{}', {}, datetime('now'))",
            escaped, batch
        ),
        _ => format!(
            "INSERT INTO migrations (migration, batch) VALUES ('{}', {})",
            escaped, batch
        ),
    }
}

fn parse_timestamp(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Migration, MigrationManager};
    use async_trait::async_trait;
    use sea_orm::{Database, DatabaseConnection};

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    async fn in_memory_db() -> DatabaseConnection {
        Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory SQLite database")
    }

    // ------------------------------------------------------------------
    // Minimal no-op migration
    // ------------------------------------------------------------------

    struct NoopMigration {
        name: &'static str,
    }

    #[async_trait]
    impl Migration for NoopMigration {
        fn name(&self) -> &str {
            self.name
        }
        async fn up(&self, _db: &DatabaseConnection) -> anyhow::Result<()> {
            Ok(())
        }
        async fn down(&self, _db: &DatabaseConnection) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn noop(name: &'static str) -> Box<dyn Migration> {
        Box::new(NoopMigration { name })
    }

    // ------------------------------------------------------------------
    // Migration that actually creates/drops a table (schema operations)
    // ------------------------------------------------------------------

    struct CreateTestTable;

    #[async_trait]
    impl Migration for CreateTestTable {
        fn name(&self) -> &str {
            "2024_01_01_000099_create_test_table"
        }

        async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                "CREATE TABLE IF NOT EXISTS test_items (
                    id    INTEGER PRIMARY KEY AUTOINCREMENT,
                    label TEXT NOT NULL
                )",
            ))
            .await?;
            Ok(())
        }

        async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
            db.execute(Statement::from_string(
                db.get_database_backend(),
                "DROP TABLE IF EXISTS test_items",
            ))
            .await?;
            Ok(())
        }
    }

    // ------------------------------------------------------------------
    // Migration that always fails in `up`
    // ------------------------------------------------------------------

    struct FailingMigration;

    #[async_trait]
    impl Migration for FailingMigration {
        fn name(&self) -> &str {
            "2024_01_01_999999_always_fails"
        }
        async fn up(&self, _db: &DatabaseConnection) -> anyhow::Result<()> {
            anyhow::bail!("intentional failure");
        }
        async fn down(&self, _db: &DatabaseConnection) -> anyhow::Result<()> {
            Ok(())
        }
    }

    // ==================================================================
    // Tests
    // ==================================================================

    #[tokio::test]
    async fn test_new_manager_has_no_migrations() {
        let db = in_memory_db().await;
        let mgr = MigrationManager::new(db);
        assert_eq!(mgr.migrations.len(), 0);
    }

    #[tokio::test]
    async fn test_register_single_migration() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_first"));
        assert_eq!(mgr.migrations.len(), 1);
    }

    #[tokio::test]
    async fn test_register_many_migrations() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_first"),
            noop("2024_01_01_000002_second"),
            noop("2024_01_01_000003_third"),
        ]);
        assert_eq!(mgr.migrations.len(), 3);
    }

    #[tokio::test]
    async fn test_ensure_migrations_table_is_idempotent() {
        let db = in_memory_db().await;
        let mgr = MigrationManager::new(db);
        // Call twice — should not fail.
        mgr.ensure_migrations_table().await.unwrap();
        mgr.ensure_migrations_table().await.unwrap();
    }

    #[tokio::test]
    async fn test_run_applies_pending_migrations() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_a"),
            noop("2024_01_01_000002_b"),
        ]);

        let result = mgr.run().await.unwrap();
        assert_eq!(result.applied.len(), 2);
        assert!(result.failed.is_empty());
        assert_eq!(result.batch, 1);
    }

    #[tokio::test]
    async fn test_run_twice_is_idempotent() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));

        mgr.run().await.unwrap();
        let second = mgr.run().await.unwrap();

        // Nothing new to apply.
        assert_eq!(second.applied.len(), 0);
        assert!(second.is_ok());
    }

    #[tokio::test]
    async fn test_run_increments_batch_for_new_migrations() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));
        mgr.run().await.unwrap(); // batch 1

        mgr.register(noop("2024_01_01_000002_b"));
        let second = mgr.run().await.unwrap(); // batch 2

        assert_eq!(second.batch, 2);
        assert_eq!(second.applied.len(), 1);
    }

    #[tokio::test]
    async fn test_status_shows_pending_and_applied() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_a"),
            noop("2024_01_01_000002_b"),
        ]);

        // Before run — all pending.
        let before = mgr.status().await.unwrap();
        assert_eq!(before.len(), 2);
        assert!(!before[0].applied);
        assert!(!before[1].applied);

        mgr.run().await.unwrap();

        // After run — all applied.
        let after = mgr.status().await.unwrap();
        assert!(after[0].applied);
        assert!(after[1].applied);
        assert_eq!(after[0].batch, Some(1));
        assert_eq!(after[1].batch, Some(1));
    }

    #[tokio::test]
    async fn test_rollback_last_batch() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_a"),
            noop("2024_01_01_000002_b"),
        ]);

        mgr.run().await.unwrap(); // batch 1

        let rb = mgr.rollback(1).await.unwrap();
        assert_eq!(rb.applied.len(), 2);
        assert!(rb.is_ok());

        // All should now be pending again.
        let status = mgr.status().await.unwrap();
        assert!(!status[0].applied);
        assert!(!status[1].applied);
    }

    #[tokio::test]
    async fn test_rollback_steps() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));
        mgr.run().await.unwrap(); // batch 1

        mgr.register(noop("2024_01_01_000002_b"));
        mgr.run().await.unwrap(); // batch 2

        // Roll back only batch 2.
        let rb = mgr.rollback(1).await.unwrap();
        assert_eq!(rb.applied.len(), 1);

        let status = mgr.status().await.unwrap();
        assert!(status[0].applied, "batch-1 migration should still be applied");
        assert!(!status[1].applied, "batch-2 migration should be rolled back");
    }

    #[tokio::test]
    async fn test_reset_rolls_back_all() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_a"),
            noop("2024_01_01_000002_b"),
        ]);

        mgr.run().await.unwrap();
        mgr.register(noop("2024_01_01_000003_c"));
        mgr.run().await.unwrap(); // two batches now

        mgr.reset().await.unwrap();

        let status = mgr.status().await.unwrap();
        for s in &status {
            assert!(!s.applied, "{} should not be applied after reset", s.name);
        }
    }

    #[tokio::test]
    async fn test_reset_on_empty_db_returns_ok() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));

        // No migrations applied yet — reset should succeed with 0 reversals.
        let result = mgr.reset().await.unwrap();
        assert_eq!(result.applied.len(), 0);
    }

    #[tokio::test]
    async fn test_fresh_reapplies_all() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register_many(vec![
            noop("2024_01_01_000001_a"),
            noop("2024_01_01_000002_b"),
        ]);

        mgr.run().await.unwrap();

        let fresh = mgr.fresh().await.unwrap();
        assert_eq!(fresh.applied.len(), 2);
        assert_eq!(fresh.batch, 1); // fresh restarts from batch 1

        let status = mgr.status().await.unwrap();
        assert!(status[0].applied);
        assert!(status[1].applied);
    }

    #[tokio::test]
    async fn test_refresh_rolls_back_last_batch_and_reruns() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));
        mgr.run().await.unwrap();

        mgr.register(noop("2024_01_01_000002_b"));
        mgr.run().await.unwrap(); // batch 2

        // refresh: rolls back batch 2 then re-runs it
        let refreshed = mgr.refresh().await.unwrap();
        assert_eq!(refreshed.applied.len(), 1); // only b was re-run

        let status = mgr.status().await.unwrap();
        assert!(status[0].applied);
        assert!(status[1].applied);
    }

    #[tokio::test]
    async fn test_rollback_with_nothing_applied_returns_error() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));

        let err = mgr.rollback(1).await.unwrap_err();
        assert!(
            matches!(err, crate::MigrationError::NothingToRollback),
            "Expected NothingToRollback, got {:?}",
            err
        );
    }

    #[tokio::test]
    async fn test_schema_migration_up_and_down() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(Box::new(CreateTestTable));

        // up
        let run = mgr.run().await.unwrap();
        assert_eq!(run.applied.len(), 1);
        assert!(run.is_ok());

        // down
        let rb = mgr.rollback(1).await.unwrap();
        assert_eq!(rb.applied.len(), 1);
        assert!(rb.is_ok());
    }

    #[tokio::test]
    async fn test_failing_migration_is_recorded_in_result() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(Box::new(FailingMigration));

        let result = mgr.run().await.unwrap();
        assert_eq!(result.applied.len(), 0);
        assert_eq!(result.failed.len(), 1);
        assert!(!result.is_ok());
        assert!(result.failed[0].1.contains("intentional failure"));
    }

    #[tokio::test]
    async fn test_run_result_display() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));

        let result = mgr.run().await.unwrap();
        let s = result.to_string();
        assert!(s.contains("Batch 1"));
        assert!(s.contains("1 applied"));
    }

    #[tokio::test]
    async fn test_migration_status_display_applied() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));
        mgr.run().await.unwrap();

        let statuses = mgr.status().await.unwrap();
        let s = statuses[0].to_string();
        assert!(s.contains("[✓]"));
        assert!(s.contains("2024_01_01_000001_a"));
        assert!(s.contains("batch: 1"));
    }

    #[tokio::test]
    async fn test_migration_status_display_pending() {
        let db = in_memory_db().await;
        let mut mgr = MigrationManager::new(db);
        mgr.register(noop("2024_01_01_000001_a"));

        let statuses = mgr.status().await.unwrap();
        let s = statuses[0].to_string();
        assert!(s.contains("[ ]"));
        assert!(s.contains("pending"));
    }

    #[tokio::test]
    async fn test_connection_accessor() {
        let db = in_memory_db().await;
        let mgr = MigrationManager::new(db);
        // Just ensure we can obtain a reference — ping proves it's live.
        mgr.connection().ping().await.unwrap();
    }
}
