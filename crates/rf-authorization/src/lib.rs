//! # rf-authorization: Laravel-style Authorization for Rust
//!
//! Comprehensive authorization system with Gates, Policies, Middleware, and database-backed permissions.
//!
//! ## Features
//!
//! - **Gates**: Simple closure-based authorization with callback support
//! - **Policies**: Class-based authorization logic for models
//! - **Middleware**: Protect routes with authorization checks
//! - **Permissions**: Database-backed RBAC (Role-Based Access Control)
//! - **Authorizable Trait**: Easy integration with models
//!
//! ## Quick Start
//!
//! ### Gates - Simple Permission Checks
//!
//! ```rust
//! use rf_authorization::gates::Gate;
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct User {
//!     is_admin: bool,
//!     permissions: Vec<String>,
//! }
//!
//! impl User {
//!     fn has_permission(&self, p: &str) -> bool {
//!         self.permissions.contains(&p.to_string())
//!     }
//! }
//!
//! let mut gate = Gate::new();
//!
//! // Define gates
//! gate.define("create-post", Arc::new(|user: &User, _| {
//!     user.is_admin || user.has_permission("create-post")
//! }));
//!
//! let admin = User { is_admin: true, permissions: vec![] };
//! assert!(gate.allows(&admin, "create-post"));
//!
//! // Or throw error
//! gate.authorize(&admin, "create-post").unwrap();
//! ```
//!
//! ### Policies - Model-Based Authorization
//!
//! ```rust
//! use rf_authorization::policies::{Policy, PolicyRegistry};
//!
//! #[derive(Clone)]
//! struct User {
//!     id: i64,
//!     is_admin: bool,
//! }
//!
//! struct Post {
//!     id: i64,
//!     author_id: i64,
//! }
//!
//! struct PostPolicy;
//!
//! impl Policy<Post> for PostPolicy {
//!     type User = User;
//!
//!     fn update(&self, user: &User, post: &Post) -> bool {
//!         user.id == post.author_id || user.is_admin
//!     }
//!
//!     fn delete(&self, user: &User, _post: &Post) -> bool {
//!         user.is_admin
//!     }
//! }
//!
//! let mut registry = PolicyRegistry::new();
//! registry.register::<Post, PostPolicy>(PostPolicy);
//!
//! let user = User { id: 1, is_admin: false };
//! let post = Post { id: 1, author_id: 1 };
//!
//! assert!(registry.authorize(&user, "update", Some(&post)).is_ok());
//! ```
//!
//! ### Database-Backed Permissions (RBAC)
//!
//! ```rust
//! use rf_authorization::permissions::{Permission, Role, UserPermissions, HasPermissions};
//!
//! let admin_role = Role::new(1, "admin")
//!     .with_permissions(vec![
//!         Permission::new(1, "posts.create"),
//!         Permission::new(2, "posts.delete"),
//!         Permission::new(3, "users.manage"),
//!     ]);
//!
//! let user_permissions = UserPermissions::from_roles(vec![admin_role]);
//!
//! assert!(user_permissions.has("posts.create"));
//! assert!(user_permissions.has("users.manage"));
//! ```
//!
//! ### Middleware - Route Protection
//!
//! ```rust
//! use rf_authorization::gates::Gate;
//! use rf_authorization::middleware::{AuthorizeGateMiddleware, Middleware, Request};
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct User { is_admin: bool }
//!
//! # tokio_test::block_on(async {
//! let mut gate = Gate::new();
//! gate.define("admin", Arc::new(|user: &User, _| user.is_admin));
//!
//! let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");
//! let request = Request::new().with_user(User { is_admin: true });
//!
//! let response = middleware.handle(request).await;
//! assert!(response.is_ok());
//! # });
//! ```

pub mod authorizable;
pub mod error;
pub mod gate;
pub mod gates;
pub mod middleware;
pub mod permissions;
pub mod policies;
pub mod policy;

pub use authorizable::Authorizable;
pub use error::{AuthorizationError, AuthorizationResult};
pub use gate::Gate;
pub use policy::Policy;
