//! Authorization module for RustForge
//!
//! This module provides a complete authorization system with policies and gates.
//!
//! # Features
//!
//! - **Policies**: Resource-based authorization with granular control
//! - **Gates**: Simple ability-based authorization
//! - **Middleware**: Protect routes with authorization checks
//! - **Extractors**: Integrate authorization into handlers
//! - **Traits**: Add authorization methods to your user types
//!
//! # Quick Start
//!
//! ## Using Policies
//!
//! Policies determine what actions a user can perform on specific resources:
//!
//! ```rust
//! use rf_auth::authorization::{
//!     policies::Policy,
//!     registry::global_registry,
//!     authorizable::Authorizable,
//! };
//! use async_trait::async_trait;
//!
//! // Your domain types
//! struct User { id: i64 }
//! struct Post { user_id: i64 }
//!
//! // Define a policy for Post
//! struct PostPolicy;
//!
//! #[async_trait]
//! impl Policy<User, Post> for PostPolicy {
//!     async fn update(&self, user: &User, post: &Post) -> bool {
//!         user.id == post.user_id
//!     }
//! }
//!
//! // Add authorization methods to your user type
//! impl Authorizable for User {}
//!
//! # async fn example() {
//! // Register the policy
//! {
//!     let mut registry = global_registry().lock().unwrap();
//!     registry.register::<User, Post, _>(PostPolicy);
//! }
//!
//! let user = User { id: 1 };
//! let post = Post { user_id: 1 };
//!
//! // Check authorization
//! if user.can("update", &post).await {
//!     // User can update the post
//! }
//!
//! // Or use authorize to get a Result
//! user.authorize("update", &post).await.expect("Not authorized");
//! # }
//! ```
//!
//! ## Using Gates
//!
//! Gates are simple closures that check general abilities:
//!
//! ```rust
//! use rf_auth::authorization::gates::Gate;
//!
//! # async fn example() {
//! #[derive(Clone)]
//! struct User { role: String }
//!
//! let gate: Gate<User> = Gate::new();
//!
//! // Define gates
//! gate.define("admin", |user| {
//!     let role = user.role.clone();
//!     async move { role == "admin" }
//! });
//!
//! gate.define("edit-posts", |user| {
//!     let role = user.role.clone();
//!     async move { role == "admin" || role == "editor" }
//! });
//!
//! let admin = User { role: "admin".to_string() };
//!
//! // Check authorization
//! if gate.allows(&admin, "admin").await {
//!     // User is an admin
//! }
//!
//! // Or use authorize to get a Result
//! gate.authorize(&admin, "edit-posts").await.expect("Not authorized");
//! # }
//! ```
//!
//! ## Middleware
//!
//! Protect routes with middleware:
//!
//! ```rust
//! use axum::{Router, routing::get};
//! use rf_auth::authorization::auth_middleware::{
//!     AuthorizeGateLayer,
//!     AuthorizePolicyLayer,
//! };
//!
//! # async fn admin_handler() -> &'static str { "Admin" }
//! # async fn update_post_handler() -> &'static str { "Updated" }
//! # fn example() {
//! #[derive(Clone)]
//! struct User;
//! #[derive(Clone)]
//! struct Post;
//!
//! let app: Router = Router::new()
//!     // Protect with a gate
//!     .route("/admin", get(admin_handler))
//!     .layer(AuthorizeGateLayer::<User>::new("admin"))
//!     // Protect with a policy
//!     .route("/posts/:id", get(update_post_handler))
//!     .layer(AuthorizePolicyLayer::<User, Post>::new("update"));
//! # }
//! ```

pub mod auth_middleware;
pub mod authorizable;
pub mod error;
pub mod gates;
pub mod integration;
pub mod policies;
pub mod registry;

// Re-export main types
pub use auth_middleware::{
    require_gate, require_policy, AuthorizeGateLayer, AuthorizeGateMiddleware,
    AuthorizePolicyLayer, AuthorizePolicyMiddleware,
};
pub use authorizable::Authorizable;
pub use error::{AuthorizationError, AuthorizationResult};
pub use gates::Gate;
pub use integration::{Authorize, AuthorizedResource, Can, CanCreate, RequireGate};
pub use policies::{Policy, PolicyCheck};
pub use registry::{global_registry, PolicyRegistry};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        auth_middleware::{require_gate, require_policy, AuthorizeGateLayer, AuthorizePolicyLayer},
        authorizable::Authorizable,
        error::{AuthorizationError, AuthorizationResult},
        gates::Gate,
        integration::{Authorize, Can, CanCreate, RequireGate},
        policies::{Policy, PolicyCheck},
        registry::{global_registry, PolicyRegistry},
    };
}
