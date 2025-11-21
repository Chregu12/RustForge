//! # rf-orm Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-orm.
//!
//! ## Usage
//!
//! ```rust
//! use rf_orm::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: collection::{Collection, IntoCollection};
pub use crate:: config::DatabaseConfig;
pub use crate:: error::{DbError, DbResult};
pub use crate:: events::{EventObserver, ModelEvent, ModelEvents};
pub use crate:: manager::DatabaseManager;
pub use crate:: migrations::{
pub use crate:: model::Model;
pub use crate:: polymorphic::{
pub use crate:: query::{
pub use crate:: query_builder::QueryBuilder;
