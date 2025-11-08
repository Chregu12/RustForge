//! Authorization System - Gates & Policies
//!
//! Provides a flexible authorization system for controlling access to resources.
//!
//! # Features
//! - Gate-based authorization for general abilities
//! - Policy-based authorization for resource-specific permissions
//! - Before/after hooks for global authorization logic
//! - Super admin bypass support
//! - Guest user checks
//!
//! # Example
//!
//! ```rust,ignore
//! // Define a gate
//! Gate::define("edit-post", |user: &User, post: &Post| {
//!     user.id == post.author_id || user.is_admin()
//! });
//!
//! // Check authorization
//! if Gate::allows("edit-post", (&user, &post)) {
//!     // User can edit post
//! }
//!
//! // Use in middleware
//! async fn edit_post(
//!     RequireAuth(user): RequireAuth,
//!     Path(post_id): Path<i64>,
//! ) -> Result<impl IntoResponse> {
//!     let post = Post::find(post_id).await?;
//!
//!     Gate::authorize("edit-post", (&user, &post))?;
//!
//!     // Perform edit
//!     Ok(StatusCode::OK)
//! }
//! ```

pub mod gate;
pub mod policy;

pub use gate::{Gate, GateRegistry, GateCallback};
pub use policy::{Policy, PolicyRegistry, ResourcePolicy};

use thiserror::Error;

/// Authorization errors
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Access denied")]
    AccessDenied,

    #[error("Gate not found: {0}")]
    GateNotFound(String),

    #[error("Policy not found for type: {0}")]
    PolicyNotFound(String),

    #[error("Guest users are not allowed")]
    GuestNotAllowed,
}

pub type AuthorizationResult<T = ()> = Result<T, AuthorizationError>;
