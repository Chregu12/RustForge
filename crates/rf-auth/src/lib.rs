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
        middleware::{auth_layer, auth_middleware, require_role},
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
