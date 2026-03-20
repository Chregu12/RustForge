//! Email verification system
//!
//! Provides email verification with JWT tokens and configurable expiration.
//!
//! # Example
//!
//! ```no_run
//! use rf_auth::verification::{EmailVerification, Verifiable};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create verification manager
//! let verification = EmailVerification::new(
//!     "your-secret-key-min-32-characters".to_string(),
//!     Duration::from_secs(24 * 60 * 60), // 24 hours
//! );
//!
//! // Generate verification URL
//! let url = verification.generate_url(
//!     "https://example.com",
//!     123,
//!     "user@example.com"
//! )?;
//!
//! // Verify token later
//! let claims = verification.verify_token(&token)?;
//! # Ok(())
//! # }
//! ```

mod middleware;
mod token;
pub mod handlers;
pub mod routes;

pub use middleware::RequireVerified;
pub use token::{EmailVerification, Verifiable, VerificationClaims};
pub use routes::verification_routes;
