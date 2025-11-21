//! # rf-auth Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-auth.
//!
//! ## Usage
//!
//! ```rust
//! use rf_auth::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: error::{AuthError, AuthResult};
pub use crate:: jwt::{Claims, JwtManager};
pub use crate:: password::{HashAlgorithm, PasswordHasher};
pub use crate:: authorization::{
pub use crate:: password_reset::{PasswordReset, ResetClaims, Resettable};
pub use crate:: remember_me::{RememberClaims, RememberMe, RememberMeMiddleware};
pub use crate:: verification::{EmailVerification, RequireVerified, VerificationClaims, Verifiable};
