//! # rf-db-facade
//!
//! Laravel-style DB facade for the RustForge framework.
//!
//! # Recommended Usage
//!
//! Use the consolidated `rf` crate for simpler imports (`use rf::DB;`),
//! or import directly from this crate:
//! ```rust
//! use rf_db_facade::DB;
//! ```
//!
//! ## Features
//!
//! - **Static DB API**: Use `DB::select()`, `DB::insert()`, etc.
//! - **Query Builder**: Chain methods like Laravel's query builder
//! - **Model Trait**: Use `User::filter()`, `User::find()` like Laravel Eloquent!
//! - **Global Connection Pool**: Thread-safe global database state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start — the `DB` query builder (this crate's real export)
//!
//! ```rust,no_run
//! use rf_db_facade::DB;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Chain queries like Laravel's query builder:
//! let admins = DB::table("users")
//!     .filter("role", "admin")
//!     .filter("active", true)
//!     .order_by("name", "asc")
//!     .limit(10)
//!     .get().await?;
//! # let _ = admins;
//! # Ok(())
//! # }
//! ```
//!
//! ## Laravel-style models
//!
//! Models are defined with the `Model!` macro (from the `rf` prelude) and then
//! expose Eloquent-style static methods. The macro needs the ORM prelude in
//! scope, so this snippet is illustrative rather than compiled here:
//!
//! ```ignore
//! use rf::prelude::*;
//!
//! Model!(User: name, email);
//!
//! let users = User::filter("active", true).get().await?;
//! let user  = User::find(1).await?;
//! let new   = User::create(serde_json::json!({ "name": "John" })).await?;
//! ```

// This crate used to carry its OWN duplicate DB/DBManager/Model/QueryBuilder
// (~2400 lines) whose query builder returned mock data (`get()` -> Ok(vec![])),
// so the Model!/create!/find! macros that expand to `rf_db_facade::Model` /
// `rf_db_facade::QueryBuilder` never touched a real database. It now re-exports
// the single real implementation from `rf_orm::facade`, so those macros execute
// real SQL through the same rusqlite-backed manager as `rf::DB`.
pub use rf_orm::facade::db::DB;
pub use rf_orm::facade::db_manager::{DBManager, GLOBAL_DB};
pub use rf_orm::facade::model::Model;
pub use rf_orm::facade::query_builder::{PaginatedResult, QueryBuilder};

// Re-export commonly used types
pub use serde_json::Value;
