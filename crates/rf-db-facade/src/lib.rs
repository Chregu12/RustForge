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
//! ## Quick Start
//!
//! ```rust,no_run
//! // Recommended: use rf::{DB, Model, model};
//! use rf_db_facade::{DB, Model, model};  // Direct import also works
//!
//! // Define models using the macro - just like Laravel!
//! model!(User, "users");
//! model!(Post, "posts");
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Laravel-style Model queries!
//! let users = User::filter("active", true).get().await?;
//! let user = User::find(1).await?;
//! let new_user = User::create(serde_json::json!({
//!     "name": "John",
//!     "email": "john@example.com"
//! })).await?;
//!
//! // Chain queries like Laravel!
//! let admins = User::filter("role", "admin")
//!     .filter("active", true)
//!     .order_by("name", "asc")
//!     .limit(10)
//!     .get().await?;
//!
//! // Or use DB::table() for raw queries
//! let users = DB::table("users")
//!     .filter("active", true)
//!     .limit(10)
//!     .get().await?;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;
pub mod model;
pub mod query_builder;

pub use facade::DB;
pub use manager::{DBManager, GLOBAL_DB};
pub use model::Model;
pub use query_builder::{QueryBuilder, PaginatedResult};

// Re-export the typed DB error from rf-orm so facade consumers can name it
pub use rf_orm::{DbError, DbResult};

// Re-export commonly used types
pub use serde_json::Value;
