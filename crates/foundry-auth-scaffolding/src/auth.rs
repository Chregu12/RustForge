//! Authentication Service
//!
//! Core authentication logic for user registration, login, and verification

use crate::models::{User, Session};
use crate::AuthConfig;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use thiserror::Error;

/// Authentication Error
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Email already exists")]
    EmailAlreadyExists,

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    #[error("Invalid two-factor code")]
    InvalidTwoFactorCode,

    #[error("Password hash error: {0}")]
    PasswordHashError(String),

    #[error("Session not found")]
    SessionNotFound,

    #[error("Session expired")]
    SessionExpired,

    #[error("Internal error: {0}")]
    InternalError(String),
}

pub type AuthResult<T> = Result<T, AuthError>;

/// Login Credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub remember: bool,
}

/// Registration Data
#[derive(Debug, Clone)]
pub struct RegisterData {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

impl RegisterData {
    /// Validate password confirmation matches
    pub fn validate(&self) -> AuthResult<()> {
        if self.password != self.password_confirmation {
            return Err(AuthError::InternalError(
                "Passwords do not match".to_string(),
            ));
        }

        if self.password.len() < 8 {
            return Err(AuthError::InternalError(
                "Password must be at least 8 characters".to_string(),
            ));
        }

        Ok(())
    }
}

/// Authentication Service
pub struct AuthService {
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// Hash a password using Argon2
    pub fn hash_password(&self, password: &str) -> AuthResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Register a new user
    pub fn register(&self, data: RegisterData) -> AuthResult<User> {
        // Validate
        data.validate()?;

        // Hash password
        let password_hash = self.hash_password(&data.password)?;

        // Create user
        Ok(User::new(data.name, data.email, password_hash))
    }

    /// Attempt to authenticate a user
    pub fn attempt(&self, user: &User, credentials: &Credentials) -> AuthResult<()> {
        // Verify password
        if !self.verify_password(&credentials.password, &user.password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        // Check email verification if required
        if self.config.require_email_verification && !user.has_verified_email() {
            return Err(AuthError::EmailNotVerified);
        }

        // Check if two-factor is enabled
        if user.has_two_factor() {
            // This would be handled separately in the two-factor flow
            // For now, we indicate it's required
            return Err(AuthError::TwoFactorRequired);
        }

        Ok(())
    }

    /// Create a session for a user
    pub fn create_session(&self, user: &User, remember: bool) -> Session {
        let lifetime = if remember {
            self.config.remember_lifetime
        } else {
            self.config.session_lifetime
        };

        let token = self.generate_session_token();
        Session::new(user.id, token, lifetime)
    }

    /// Generate a secure random session token
    fn generate_session_token(&self) -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(64)
            .collect();

        URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    /// Generate a secure random token (for password reset, email verification, etc.)
    pub fn generate_token(&self) -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(32)
            .collect();

        URL_SAFE_NO_PAD.encode(&random_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let service = AuthService::new(AuthConfig::default());

        let password = "my_secure_password_123";
        let hash = service.hash_password(password).unwrap();

        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_register_user() {
        let service = AuthService::new(AuthConfig::default());

        let data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "password123".to_string(),
            password_confirmation: "password123".to_string(),
        };

        let user = service.register(data).unwrap();
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
    }

    #[test]
    fn test_register_validation() {
        let service = AuthService::new(AuthConfig::default());

        // Mismatched passwords
        let data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "password123".to_string(),
            password_confirmation: "different".to_string(),
        };

        assert!(service.register(data).is_err());

        // Short password
        let data = RegisterData {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "short".to_string(),
            password_confirmation: "short".to_string(),
        };

        assert!(service.register(data).is_err());
    }

    #[test]
    fn test_authentication() {
        let service = AuthService::new(AuthConfig::default());

        let password_hash = service.hash_password("password123").unwrap();
        let mut user = User::new("John".to_string(), "john@example.com".to_string(), password_hash);
        user.mark_email_as_verified();

        let credentials = Credentials {
            email: "john@example.com".to_string(),
            password: "password123".to_string(),
            remember: false,
        };

        assert!(service.attempt(&user, &credentials).is_ok());

        let bad_credentials = Credentials {
            email: "john@example.com".to_string(),
            password: "wrong_password".to_string(),
            remember: false,
        };

        assert!(service.attempt(&user, &bad_credentials).is_err());
    }

    #[test]
    fn test_session_creation() {
        let service = AuthService::new(AuthConfig::default());

        let user = User::new(
            "John".to_string(),
            "john@example.com".to_string(),
            "hash".to_string(),
        );

        let session = service.create_session(&user, false);
        assert_eq!(session.user_id, user.id);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_token_generation() {
        let service = AuthService::new(AuthConfig::default());

        let token1 = service.generate_token();
        let token2 = service.generate_token();

        assert_ne!(token1, token2);
        assert!(!token1.is_empty());
    }
}
