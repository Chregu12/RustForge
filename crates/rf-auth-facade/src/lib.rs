//! # rf-auth-facade
//!
//! Laravel-style `Auth` facade for the RustForge framework.
//!
//! This crate used to carry its **own** duplicate `Auth`/`AuthManager`/`Guard`
//! implementation. That duplicate held the current user in a single process-global
//! (a cross-request state leak) and shipped a mock `attempt` that logged anyone in.
//! It now simply **re-exports the single, request-scoped implementation from
//! [`rf_auth`]**, so there is exactly one correct source of truth. Establish a
//! per-request scope with `rf_auth::middleware::auth_scope`.
//!
//! # Recommended Usage
//!
//! Prefer the consolidated `rf` crate (`use rf::Auth;` / `use rf::prelude::*;`).
//! When depending on this crate directly:
//!
//! ```rust
//! use rf_auth_facade::Auth;
//! ```

// One source of truth: the request-scoped auth implementation in `rf-auth`.
pub use rf_auth::{
    with_auth_scope, with_auth_scope_sync, Auth, AuthError, AuthManager, AuthResult, Claims, Guard,
    JwtManager, UserProvider, GLOBAL_AUTH,
};
