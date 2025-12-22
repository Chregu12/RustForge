//! # rf-core: Core Foundation for RustForge Framework
//!
//! This crate provides the foundational types for the RustForge framework:
//! - RFC 7807 Problem Details error handling
//! - Request context with trace IDs
//! - Type-safe error types
//!
//! ## Example
//!
//! ```rust
//! use rf_core::{AppError, AppResult, RequestContext};
//!
//! fn get_user(id: i32, ctx: &RequestContext) -> AppResult<User> {
//!     if id <= 0 {
//!         return Err(AppError::BadRequest {
//!             message: "ID must be positive".to_string(),
//!         });
//!     }
//!
//!     // Find user...
//!     User::find(id)
//!         .ok_or_else(|| AppError::NotFound {
//!             resource: format!("User {}", id),
//!         })
//! }
//! # #[derive(Debug)]
//! # struct User;
//! # impl User {
//! #     fn find(id: i32) -> Option<Self> { None }
//! # }
//! ```

pub mod context;
pub mod error;
pub mod prelude;
pub mod runtime;

// Axum support (optional)
#[cfg(feature = "axum")]
pub mod axum_support;

// Re-exports for convenience
pub use context::{Environment, RequestContext};
pub use error::{AppError, AppResult, ProblemDetails};
