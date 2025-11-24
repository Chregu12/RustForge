//! # rf-db-facade
//!
//! Laravel-style DB facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static DB API**: Use `DB::select()`, `DB::insert()`, etc.
//! - **Query Builder**: Chain methods like Laravel's query builder
//! - **Global Connection Pool**: Thread-safe global database state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_db_facade::DB;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Execute a select query
//! let users = DB::select("SELECT * FROM users WHERE active = ?", &[true.into()]).await?;
//!
//! // Insert a record
//! let id = DB::insert("INSERT INTO users (name, email) VALUES (?, ?)",
//!     &["John".into(), "john@example.com".into()]).await?;
//!
//! // Update records
//! let affected = DB::update("UPDATE users SET active = ? WHERE id = ?",
//!     &[true.into(), id.into()]).await?;
//!
//! // Delete records
//! let deleted = DB::delete("DELETE FROM users WHERE id = ?", &[id.into()]).await?;
//!
//! // Use query builder
//! let users = DB::table("users")
//!     .where_clause("active", "=", true.into())
//!     .limit(10)
//!     .get().await?;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;
pub mod query_builder;

pub use facade::DB;
pub use manager::{DBManager, GLOBAL_DB};
pub use query_builder::QueryBuilder;

// Re-export commonly used types
pub use serde_json::Value;
