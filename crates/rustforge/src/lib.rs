//! # RustForge - The Laravel of Rust
//!
//! Import everything you need with a single line:
//!
//! ```rust,ignore
//! use rustforge::*;
//!
//! Model!(User: name, email, hidden password);
//!
//! // Use query! macro to write `where` like Laravel!
//! let users = query!(User::where("active", true).get()).await;
//! let admins = query! {
//!     User::where("role", "admin")
//!         .where("active", true)
//!         .orderBy("name", "asc")
//!         .get()
//! }.await;
//! ```
//!
//! With `#[auto_await]` - no `.await` needed:
//!
//! ```rust,ignore
//! use rustforge::*;
//!
//! Model!(User: name, email, hidden password);
//!
//! #[auto_await]
//! async fn example() -> Result<()> {
//!     let users = query!(User::where("active", true).get());
//!     let user = User::find(1);
//!     Ok(())
//! }
//! ```

// ============================================================================
// Macros - These are automatically available after `use rustforge::*;`
// ============================================================================

/// Model macro - define models like Laravel's Eloquent
///
/// ```rust
/// use rustforge::*;
///
/// #[model]
/// pub struct User {
///     pub name: String,
///     pub email: String,
///     #[hidden]
///     pub password: String,
/// }
/// ```
pub use rf_model_macro::model;

/// Laravel-like class syntax for models
///
/// ```rust
/// use rustforge::*;
///
/// laravel! {
///     class User extends Model {
///         protected fillable = [name: String, email: String];
///         protected hidden = [password: String];
///     }
/// }
/// ```
pub use rf_macros::laravel;

/// Ultra-simple model macro - the cleanest syntax!
///
/// ```rust
/// use rustforge::*;
///
/// // Minimal - all fields are String by default
/// Model!(User: name, email, hidden password);
///
/// // Or with explicit types
/// Model!(Post {
///     title: String,
///     body: String,
///     user_id: i64,
/// });
/// ```
#[allow(non_snake_case)]
pub use rf_macros::Model;

/// Query macro - use `where` like Laravel!
///
/// In Rust, `where` is a reserved keyword. This macro lets you use it anyway:
///
/// ```rust
/// use rustforge::*;
///
/// let users = query!(User::where("active", true).get()).await;
///
/// let admins = query! {
///     User::where("role", "admin")
///         .where("active", true)
///         .orderBy("name", "asc")
///         .limit(10)
///         .get()
/// }.await;
/// ```
pub use rf_macros::query;

/// Auto-await macro - write async code without explicit .await
///
/// ```rust
/// use rustforge::*;
///
/// #[auto_await]
/// async fn handler() -> Result<Response, Error> {
///     let users = User::filter("active", true).get();  // No .await needed!
///     Ok(Response::json(users))
/// }
/// ```
pub use rf_macros::auto_await;

/// Controller macro
pub use rf_macros::controller;

/// Function macro for inline handlers
pub use rf_macros::function;

/// Validation rules macro
pub use rf_macros::rules;

// ============================================================================
// Facades - Static API like Laravel
// ============================================================================

/// Authentication facade
///
/// ```rust
/// Auth::attempt(json!({"email": "...", "password": "..."}));
/// let user = Auth::user::<User>();
/// Auth::logout();
/// ```
pub use rf_auth_facade::Auth;

/// Cache facade
///
/// ```rust
/// Cache::put("key", "value", 3600);
/// let value = Cache::get("key");
/// Cache::forget("key");
/// ```
pub use rf_cache_facade::Cache;

/// Database facade
///
/// ```rust
/// let users = DB::table("users").filter("active", true).get();
/// ```
pub use rf_db_facade::DB;

/// Model trait for Eloquent-style queries
///
/// ```rust
/// let users = User::filter("active", true).get();
/// let user = User::find(1);
/// ```
pub use rf_db_facade::Model;

/// Event facade
pub use rf_event_facade::Event;

/// Storage facade
///
/// ```rust
/// Storage::put("file.txt", contents);
/// let data = Storage::get("file.txt");
/// ```
pub use rf_storage_facade::Storage;

/// Route facade
///
/// ```rust
/// Route::get("/users", handler);
/// Route::post("/users", create_handler);
/// ```
pub use rf_route_facade::Route;

// ============================================================================
// HTTP Types
// ============================================================================

pub use rf_request::Request;
pub use rf_response::Response;

// ============================================================================
// Validation
// ============================================================================

pub use rf_validation::Validate;

// ============================================================================
// Common Re-exports
// ============================================================================

/// JSON macro for creating JSON values
pub use serde_json::json;

/// JSON Value type
pub use serde_json::Value;

/// Serde derives
pub use serde::{Deserialize, Serialize};

/// Async trait for implementing async traits
pub use async_trait::async_trait;

// ============================================================================
// Type Aliases for cleaner code
// ============================================================================

/// Standard Result type with String error
pub type Result<T> = std::result::Result<T, String>;

/// Standard Error type
pub type Error = String;
