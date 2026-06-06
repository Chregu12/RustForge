//! # rf-migrations
//!
//! A Laravel-inspired database migration system for RustForge.
//!
//! Provides a complete migration lifecycle:
//! - `run()` — apply all pending migrations
//! - `rollback(steps)` — roll back N batches
//! - `reset()` — roll back all migrations
//! - `fresh()` — reset + run (wipe & re-apply)
//! - `refresh()` — rollback + migrate (alias for fresh but semantically "same batch")
//! - `status()` — show which migrations are applied/pending
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_migrations::{Migration, MigrationManager};
//! use async_trait::async_trait;
//! use sea_orm::{ConnectionTrait, DatabaseConnection};
//!
//! pub struct CreateUsersTable;
//!
//! #[async_trait]
//! impl Migration for CreateUsersTable {
//!     fn name(&self) -> &str {
//!         "2024_01_01_000001_create_users_table"
//!     }
//!
//!     async fn up(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
//!         db.execute(sea_orm::Statement::from_string(
//!             db.get_database_backend(),
//!             "CREATE TABLE IF NOT EXISTS users (
//!                 id INTEGER PRIMARY KEY AUTOINCREMENT,
//!                 name TEXT NOT NULL,
//!                 email TEXT NOT NULL UNIQUE,
//!                 created_at TEXT DEFAULT CURRENT_TIMESTAMP
//!             )",
//!         )).await?;
//!         Ok(())
//!     }
//!
//!     async fn down(&self, db: &DatabaseConnection) -> anyhow::Result<()> {
//!         db.execute(sea_orm::Statement::from_string(
//!             db.get_database_backend(),
//!             "DROP TABLE IF EXISTS users",
//!         )).await?;
//!         Ok(())
//!     }
//! }
//!
//! # async fn example(db: sea_orm::DatabaseConnection) -> anyhow::Result<()> {
//! let mut manager = MigrationManager::new(db);
//! manager.register(Box::new(CreateUsersTable));
//!
//! // Apply all pending migrations
//! let result = manager.run().await?;
//! println!("{}", result);
//!
//! // Check status
//! for s in manager.status().await? {
//!     println!("{}", s);
//! }
//!
//! // Rollback last batch
//! manager.rollback(1).await?;
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod manager;
pub mod migration;
pub mod record;
pub mod status;

pub use error::{MigrationError, Result};
pub use manager::MigrationManager;
pub use migration::Migration;
pub use record::RunResult;
pub use status::MigrationStatus;
