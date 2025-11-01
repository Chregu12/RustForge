//! # Foundry Audit Logging
//!
//! Complete audit trail system for tracking model changes.
//!
//! ## Features
//! - Track User, Action, Model, Changes
//! - JSON Serialization for old/new values
//! - Queryable Audit History
//! - Integration with authentication
//! - CLI commands for viewing audit logs

pub mod models;
pub mod traits;
pub mod logger;
pub mod query;

pub use models::{AuditLog, AuditAction};
pub use traits::{Auditable, AuditContext};
pub use logger::AuditLogger;
pub use query::AuditQuery;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid audit log")]
    InvalidAuditLog,
}

pub type Result<T> = std::result::Result<T, AuditError>;
