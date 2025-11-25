//! # RustForge - The Laravel of Rust
//!
//! Write Rust exactly like Laravel PHP!
//!
//! ```rust,ignore
//! use rustforge::*;
//!
//! Model!(User: name, email, hidden password);
//!
//! #[auto_await]  // <- Once at top, applies to entire module!
//! mod app {
//!     use super::*;
//!
//!     pub async fn index() -> Response {
//!         let users = User::where("active", true)
//!             .orderBy("name", "asc")
//!             .get();
//!         Response::json(users)
//!     }
//!
//!     pub async fn show(id: i64) -> Response {
//!         let user = User::findOrFail(id);
//!         Response::json(user)
//!     }
//!
//!     pub async fn store(data: Json<Value>) -> Response {
//!         let user = User::create(data.0);
//!         Response::json(user)
//!     }
//! }
//!
//! pub use app::*;  // Re-export everything
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
// Laravel Helper Macros
// ============================================================================

/// Create a Laravel-style collection
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let numbers = collect![1, 2, 3, 4, 5];
/// let doubled = numbers.map(|x| x * 2);
/// ```
pub use rf_macros::collect;

/// Get configuration value
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let db_host = config!("database.host");
/// let timeout = config!("cache.timeout", 3600);
/// ```
pub use rf_macros::config;

/// Get environment variable
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let env = env_var!("APP_ENV");
/// let debug = env_var!("DEBUG", "false");
/// ```
pub use rf_macros::env_var;

/// Generate named route URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let url = route!("users.show", id = 123);
/// ```
pub use rf_macros::route;

/// Create HTTP responses easily
///
/// ```rust,ignore
/// use rustforge::*;
///
/// response!(json: data)
/// response!(redirect: "/home")
/// response!(view: "welcome", data)
/// ```
pub use rf_macros::response;

/// Abort with HTTP error
///
/// ```rust,ignore
/// use rustforge::*;
///
/// abort!(404);
/// abort!(403, "Forbidden");
/// ```
pub use rf_macros::abort;

/// Dump and die - debug helper
///
/// ```rust,ignore
/// use rustforge::*;
///
/// dd!(user, request);  // Prints and exits
/// ```
pub use rf_macros::dd;

/// Dump without stopping - debug helper
///
/// ```rust,ignore
/// use rustforge::*;
///
/// dump!(user, config);  // Prints and continues
/// ```
pub use rf_macros::dump;

/// Get old form input value
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let email = old!("email");
/// let name = old!("name", "Default");
/// ```
pub use rf_macros::old;

/// Generate asset URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let css = asset!("css/app.css");
/// ```
pub use rf_macros::asset;

/// Generate full URL
///
/// ```rust,ignore
/// use rustforge::*;
///
/// let url = url!("/users/123");
/// ```
pub use rf_macros::url;

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
