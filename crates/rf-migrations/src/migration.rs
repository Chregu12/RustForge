//! The core `Migration` trait that every migration must implement.

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

/// A database migration with a forward (`up`) and reverse (`down`) operation.
///
/// # Naming convention
///
/// Use timestamps to ensure deterministic ordering:
/// `YYYY_MM_DD_HHMMSS_description`, e.g.
/// `"2024_01_15_120000_create_users_table"`.
///
/// # Example
///
/// ```rust,no_run
/// use rf_migrations::Migration;
/// use async_trait::async_trait;
/// use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
///
/// pub struct CreatePostsTable;
///
/// #[async_trait]
/// impl Migration for CreatePostsTable {
///     fn name(&self) -> &str {
///         "2024_01_15_120000_create_posts_table"
///     }
///
///     async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
///         db.execute(Statement::from_string(
///             db.get_database_backend(),
///             "CREATE TABLE IF NOT EXISTS posts (
///                 id      INTEGER PRIMARY KEY AUTOINCREMENT,
///                 title   TEXT NOT NULL,
///                 body    TEXT NOT NULL,
///                 created_at TEXT DEFAULT CURRENT_TIMESTAMP
///             )",
///         )).await?;
///         Ok(())
///     }
///
///     async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
///         db.execute(Statement::from_string(
///             db.get_database_backend(),
///             "DROP TABLE IF EXISTS posts",
///         )).await?;
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Migration: Send + Sync {
    /// Unique, stable name for this migration.
    ///
    /// This name is stored in the tracking table and used to determine whether
    /// the migration has already been applied. It must never change after the
    /// migration has been deployed.
    fn name(&self) -> &str;

    /// Apply the migration (schema up / forward).
    async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()>;

    /// Reverse the migration (schema down / rollback).
    async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()>;
}
