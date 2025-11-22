//! # Foundry Soft Deletes
//!
//! Soft delete functionality for Sea-ORM models.
//!
//! ## Features
//! - Trait for Sea-ORM Models
//! - Automatic WHERE deleted_at IS NULL
//! - Restore & ForceDelete Operations
//! - Query Scopes (withTrashed, onlyTrashed)
//! - Migration helpers
//! - Audit Trail Integration

pub mod migration;
pub mod scopes;
pub mod traits;

pub use scopes::{QueryScopeExt, SoftDeleteScope};
pub use traits::{SoftDelete, SoftDeleteExt};

use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SoftDeleteError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Already deleted")]
    AlreadyDeleted,

    #[error("Not deleted")]
    NotDeleted,
}

pub type Result<T> = std::result::Result<T, SoftDeleteError>;

/// Trait marker for models with soft delete support
pub trait HasSoftDelete {
    fn deleted_at(&self) -> Option<DateTime<Utc>>;
    fn set_deleted_at(&mut self, value: Option<DateTime<Utc>>);
}
