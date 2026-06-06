//! Error types for the migration system.

use thiserror::Error;

/// Errors that can occur during migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// A SeaORM database error.
    #[error("Database error: {0}")]
    Db(#[from] sea_orm::DbErr),

    /// A migration referenced by name could not be found in the registered list.
    #[error("Migration not found in registry: '{0}'")]
    NotFound(String),

    /// A migration failed during execution.
    #[error("Migration '{name}' failed: {source}")]
    ExecutionFailed {
        name: String,
        #[source]
        source: anyhow::Error,
    },

    /// There are no applied migrations to roll back.
    #[error("Nothing to roll back — no migrations have been applied yet")]
    NothingToRollback,

    /// An invalid operation was requested.
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Convenient `Result` alias for this crate.
pub type Result<T> = std::result::Result<T, MigrationError>;
