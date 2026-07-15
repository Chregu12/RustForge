//! # rf-auth - Authentication, Authorization & Security
//!
//! Production-ready authentication and authorization layer for the RustForge framework.
//!
//! ## Features
//!
//! ### Authentication
//! - **Password Hashing**: Secure password storage with bcrypt or argon2
//! - **JWT Tokens**: Generate and validate JSON Web Tokens
//! - **Middleware**: Axum middleware for protected routes
//! - **Claims Extraction**: Automatic JWT validation in handlers
//!
//! ### Authorization
//! - **Policies**: Resource-based authorization with fine-grained control
//! - **Gates**: Simple ability-based authorization
//! - **Policy Registry**: Type-safe policy management
//! - **Authorizable Trait**: Add authorization methods to user types
//! - **Middleware**: Protect routes with policies and gates
//! - **Extractors**: Integrate authorization into Axum handlers
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_auth::{PasswordHasher, JwtManager, Claims};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Password hashing
//! let hasher = PasswordHasher::bcrypt(12)?;
//! let hash = hasher.hash("my_password")?;
//! assert!(hasher.verify("my_password", &hash)?);
//!
//! // JWT tokens
//! let jwt = JwtManager::new("your-secret-key-min-32-characters")?;
//! let claims = Claims::new(
//!     123,
//!     "user@example.com".to_string(),
//!     vec!["user".to_string()],
//!     24, // 24 hours
//! );
//! let token = jwt.generate_token(&claims)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Axum Integration
//!
//! ```ignore
//! use rf_auth::{JwtManager, middleware::auth_layer};
//! use axum::{Router, routing::get, Json, Extension};
//! use serde_json::json;
//! use std::sync::Arc;
//!
//! async fn protected_handler() -> Json<serde_json::Value> {
//!     Json(json!({"message": "protected"}))
//! }
//!
//! let jwt = Arc::new(JwtManager::new("your-secret-key-min-32-characters")?);
//!
//! let app = Router::new()
//!     .route("/protected", get(protected_handler))
//!     .layer(Extension(jwt))
//!     .route_layer(axum::middleware::from_fn(auth_layer));
//! ```

// Authentication modules
pub mod error;
pub mod extractor;
pub mod jwt;
pub mod middleware;
pub mod password;

// Authorization module
pub mod authorization;

// Auth facade (Laravel-style static API)
pub mod auth_manager;
pub mod facade;
pub mod guard;

// Auth features (require mail feature for email sending)
#[cfg(feature = "mail")]
pub mod password_reset;
pub mod remember_me;
#[cfg(feature = "mail")]
pub mod verification;

// Re-export main authentication types
pub use error::{AuthError, AuthResult};
pub use jwt::{Claims, JwtManager};
pub use password::{HashAlgorithm, PasswordHasher};

// Re-export facade types (Laravel-style static API)
pub use facade::Auth;
pub use auth_manager::{
    in_auth_scope, with_auth_scope, with_auth_scope_sync, AuthManager, UserProvider, GLOBAL_AUTH,
};
pub use guard::Guard;

/// Ready-made JWT bearer-auth route guard. Apply it with
/// `.route_layer(axum::middleware::from_fn(require_auth))` combined with
/// `.layer(Extension(Arc::new(JwtManager::new(secret)?)))` to reject requests
/// that carry no valid JWT bearer token with a `401 Unauthorized` JSON response
/// **before** the handler and its body extractors run (so auth precedes any 422).
///
/// For apps that hold the [`JwtManager`] inside application state (rather than
/// an `Extension`), use [`require_auth_with`] instead.
///
/// ```rust,ignore
/// use rf::prelude::*;  // re-exports require_auth
/// use axum::{middleware::from_fn, Extension};
/// use rf_auth::JwtManager;
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("32-char-secret...")?);
///
/// let protected = axum::Router::new()
///     .route("/profile", axum::routing::get(profile_handler))
///     .route_layer(from_fn(require_auth))
///     .layer(Extension(jwt));
///
/// // Unauthenticated or invalid JWT: {"error":"Unauthorized"} 401
/// // Valid JWT: handler runs; Auth::user() / Auth::id() work inside
/// ```
pub use middleware::require_auth;

/// Alternative to [`require_auth`] when the [`JwtManager`] lives in app state
/// rather than an Axum `Extension`. Returns a closure ready for
/// `axum::middleware::from_fn`:
///
/// ```rust,ignore
/// use rf_auth::{require_auth_with, JwtManager};
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("32-char-secret...")?);
///
/// let protected = axum::Router::new()
///     .route("/profile", axum::routing::get(profile_handler))
///     .route_layer(axum::middleware::from_fn(require_auth_with(jwt)));
/// ```
pub use middleware::require_auth_with;

// Re-export main authorization types
pub use authorization::{
    Authorizable, AuthorizationError, AuthorizationResult, Gate, Policy, PolicyRegistry,
};

// Re-export auth features
#[cfg(feature = "mail")]
pub use password_reset::{PasswordReset, ResetClaims, Resettable};
pub use remember_me::{RememberClaims, RememberMe, RememberMeMiddleware};
#[cfg(feature = "mail")]
pub use verification::{EmailVerification, RequireVerified, Verifiable, VerificationClaims};

/// Prelude module for convenient imports
pub mod prelude {
    // Authentication
    pub use crate::{
        error::{AuthError, AuthResult},
        jwt::{Claims, JwtManager},
        middleware::{auth_layer, auth_middleware, require_auth, require_auth_with, require_role},
        password::{HashAlgorithm, PasswordHasher},
    };

    // Authorization
    pub use crate::authorization::{
        prelude::*, Authorizable, AuthorizationError, AuthorizationResult, Gate, Policy,
        PolicyRegistry,
    };

    // Auth features
    #[cfg(feature = "mail")]
    pub use crate::password_reset::{PasswordReset, ResetClaims, Resettable};
    pub use crate::remember_me::{RememberClaims, RememberMe, RememberMeMiddleware};
    #[cfg(feature = "mail")]
    pub use crate::verification::{
        EmailVerification, RequireVerified, Verifiable, VerificationClaims,
    };
}
