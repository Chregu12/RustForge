//! # Foundry Auth Scaffolding
//!
//! Complete authentication system for Rust web applications, inspired by Laravel Breeze and Jetstream.
//! Drop-in authentication with minimal boilerplate.
//!
//! ## Features
//!
//! - **User Registration & Login** - Secure password-based authentication with Argon2
//! - **Password Reset Flow** - Secure password reset via email with time-limited tokens
//! - **Email Verification** - Optional email verification for new users
//! - **Two-Factor Authentication (TOTP)** - Time-based one-time passwords with QR codes and recovery codes
//! - **Session Management** - Secure session handling with configurable lifetimes
//! - **Remember Me** - Long-lived sessions for trusted devices
//! - **Profile Management** - User profile updates and password changes
//! - **CSRF Protection** - Built-in protection against cross-site request forgery
//! - **Rate Limiting** - Prevent brute force attacks on authentication endpoints
//!
//! ## Quick Start
//!
//! ```rust
//! use foundry_auth_scaffolding::{AuthService, AuthConfig, RegisterData, Credentials};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Configure authentication
//! let config = AuthConfig {
//!     session_lifetime: 7200,  // 2 hours
//!     require_email_verification: true,
//!     enable_two_factor: true,
//!     ..Default::default()
//! };
//!
//! // 2. Create auth service
//! let auth_service = AuthService::new(config);
//!
//! // 3. Register a new user
//! let register_data = RegisterData {
//!     name: "John Doe".to_string(),
//!     email: "john@example.com".to_string(),
//!     password: "secure_password_123".to_string(),
//!     password_confirmation: "secure_password_123".to_string(),
//! };
//!
//! let user = auth_service.register(register_data)?;
//!
//! // 4. Authenticate user
//! let credentials = Credentials {
//!     email: "john@example.com".to_string(),
//!     password: "secure_password_123".to_string(),
//!     remember: false,
//! };
//!
//! auth_service.attempt(&user, &credentials)?;
//!
//! // 5. Create session
//! let session = auth_service.create_session(&user, false);
//! # Ok(())
//! # }
//! ```
//!
//! ## Security
//!
//! - Passwords hashed with Argon2 (memory-hard, resistant to GPU attacks)
//! - Constant-time comparison for sensitive operations (prevents timing attacks)
//! - Secure random token generation for password resets and email verification
//! - TOTP-based two-factor authentication with recovery codes
//! - Session tokens generated with cryptographically secure randomness
//!
//! ## Web Framework Integration
//!
//! This crate provides Axum route handlers out of the box. See [`handlers`] module for details.

pub mod models;
pub mod auth;
pub mod handlers;
pub mod middleware;
pub mod templates;
pub mod password;
pub mod session;
pub mod email_verification;
pub mod repositories;

#[cfg(feature = "two-factor")]
pub mod two_factor;

#[cfg(feature = "email")]
pub mod email;

pub use models::{User, Session};
pub use auth::{AuthService, Credentials, RegisterData};
pub use handlers::auth_routes;
pub use middleware::{RequireAuth, OptionalAuth};
pub use session::SessionManager;

/// Auth Scaffolding Configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Session lifetime in seconds (default: 7200 = 2 hours)
    pub session_lifetime: i64,

    /// Remember me lifetime in seconds (default: 2592000 = 30 days)
    pub remember_lifetime: i64,

    /// Password reset token lifetime in seconds (default: 3600 = 1 hour)
    pub password_reset_lifetime: i64,

    /// Email verification token lifetime in seconds (default: 86400 = 24 hours)
    pub email_verification_lifetime: i64,

    /// Require email verification
    pub require_email_verification: bool,

    /// Enable two-factor authentication
    pub enable_two_factor: bool,

    /// Application name (for emails and 2FA)
    pub app_name: String,

    /// Application URL (for emails)
    pub app_url: String,

    /// From email address
    pub from_email: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            session_lifetime: 7200,
            remember_lifetime: 2592000,
            password_reset_lifetime: 3600,
            email_verification_lifetime: 86400,
            require_email_verification: false,
            enable_two_factor: false,
            app_name: "Foundry Application".to_string(),
            app_url: "http://localhost:3000".to_string(),
            from_email: "noreply@example.com".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuthConfig::default();
        assert_eq!(config.session_lifetime, 7200);
        assert_eq!(config.remember_lifetime, 2592000);
        assert!(!config.require_email_verification);
    }
}
