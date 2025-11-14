//! Password reset system
//!
//! Provides secure password reset with JWT tokens and configurable expiration.
//!
//! # Security Features
//!
//! - JWT-based tokens with signature verification
//! - Configurable expiration (default: 1 hour)
//! - Token includes user_id and email for verification
//! - Supports rate limiting integration
//!
//! # Example
//!
//! ```no_run
//! use rf_auth::password_reset::{PasswordReset, Resettable};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create password reset manager
//! let reset = PasswordReset::new(
//!     "your-secret-key-min-32-characters".to_string(),
//!     Duration::from_secs(60 * 60), // 1 hour
//! );
//!
//! // Generate reset URL
//! let url = reset.generate_url(
//!     "https://example.com",
//!     123,
//!     "user@example.com"
//! )?;
//!
//! // Verify token later
//! let claims = reset.verify_token(&token)?;
//! # Ok(())
//! # }
//! ```

mod token;

pub use token::{PasswordReset, ResetClaims, Resettable};
