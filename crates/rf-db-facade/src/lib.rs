//! # rf-db-facade
//!
//! Laravel-style DB facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static DB API**: Use `DB::select()`, `DB::insert()`, etc.
//! - **Query Builder**: Chain methods like Laravel's query builder
//! - **Model Trait**: Use `User::where()`, `User::find()` like Laravel Eloquent!
//! - **Global Connection Pool**: Thread-safe global database state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_db_facade::{DB, Model, model};
//!
//! // Define models using the macro - just like Laravel!
//! model!(User, "users");
//! model!(Post, "posts");
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Laravel-style Model queries!
//! let users = User::where("active", true).get().await?;
//! let user = User::find(1).await?;
//! let new_user = User::create(serde_json::json!({
//!     "name": "John",
//!     "email": "john@example.com"
//! })).await?;
//!
//! // Chain queries like Laravel!
//! let admins = User::where("role", "admin")
//!     .where("active", true)
//!     .order_by("name", "asc")
//!     .limit(10)
//!     .get().await?;
//!
//! // Or use DB::table() for raw queries
//! let users = DB::table("users")
//!     .r#where("active", true)
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

// Re-export commonly used types
pub use serde_json::Value;
