//! Remember Me authentication system
//!
//! Provides long-lived session support with secure cookie management.
//!
//! # Security Features
//!
//! - Cryptographically secure token generation
//! - HTTP-only cookies
//! - Secure flag for HTTPS
//! - SameSite=Strict for CSRF protection
//! - Token rotation on each use
//! - Configurable expiration (default: 30 days)
//!
//! # Example
//!
//! ```no_run
//! use rf_auth::remember_me::RememberMe;
//! use std::time::Duration;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create remember me manager
//! let remember = RememberMe::new(
//!     "your-secret-key-min-32-characters".to_string(),
//!     Duration::from_secs(30 * 24 * 60 * 60), // 30 days
//! );
//!
//! // Generate token
//! let token = remember.generate_token(123)?;
//!
//! // Create secure cookie
//! let cookie = remember.create_cookie(123)?;
//! # Ok(())
//! # }
//! ```

mod cookie;
mod middleware;

pub use cookie::{RememberClaims, RememberMe};
pub use middleware::RememberMeMiddleware;
