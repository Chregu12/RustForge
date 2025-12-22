//! # rf-auth-facade
//!
//! Laravel-style Auth facade for the RustForge framework.
//!
//! This crate provides a static, fluent API for authentication similar to Laravel's Auth facade,
//! making it easy to work with authentication from anywhere in your application.
//!
//! # Recommended Usage
//!
//! Use the consolidated `rf` crate for simpler imports:
//! ```rust
//! use rf::Auth;  // or use rf::prelude::*;
//! ```
//!
//! ## Features
//!
//! - **Static Auth API**: Use `Auth::check()`, `Auth::user()`, etc. - no `.await` needed!
//! - **Global Auth Manager**: Thread-safe global authentication state
//! - **Guard Support**: Multiple authentication guards
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // Recommended: use rf::Auth;
//! use rf_auth_facade::Auth;  // Direct import also works
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct User {
//!     id: u64,
//!     email: String,
//!     name: String,
//! }
//!
//! fn example() -> Result<(), String> {
//!     // Login a user
//!     let user = User {
//!         id: 1,
//!         email: "user@example.com".to_string(),
//!         name: "John Doe".to_string(),
//!     };
//!     Auth::login(user.clone())?;
//!
//!     // Check if authenticated
//!     if Auth::check() {
//!         println!("User is authenticated");
//!     }
//!
//!     // Get current user
//!     if let Some(current_user) = Auth::user::<User>() {
//!         println!("Current user: {}", current_user.name);
//!     }
//!
//!     // Get user ID
//!     if let Some(id) = Auth::id() {
//!         println!("User ID: {}", id);
//!     }
//!
//!     // Logout
//!     Auth::logout();
//!     Ok(())
//! }
//! ```
//!
//! ## Authentication Flow
//!
//! ```rust,no_run
//! // Recommended: use rf::Auth;
//! use rf_auth_facade::Auth;  // Direct import also works
//!
//! fn example() -> Result<(), String> {
//!     // Attempt login with credentials
//!     let credentials = serde_json::json!({
//!         "email": "user@example.com",
//!         "password": "secret"
//!     });
//!
//!     if Auth::attempt(credentials)? {
//!         println!("Login successful!");
//!     } else {
//!         println!("Invalid credentials");
//!     }
//!     Ok(())
//! }
//! ```

pub mod facade;
pub mod manager;
pub mod guard;

pub use facade::Auth;
pub use manager::{AuthManager, GLOBAL_AUTH};
pub use guard::Guard;

// Re-export commonly used types from rf-auth
pub use rf_auth::{AuthError, AuthResult, Claims, JwtManager};
