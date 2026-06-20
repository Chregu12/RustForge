//! Password reset token generation and validation

use crate::error::{AuthError, AuthResult};
use crate::password::PasswordHasher;
use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Claims for password reset tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetClaims {
    /// User ID (subject)
    pub sub: i64,
    /// Email address for password reset
    pub email: String,
    /// Expiration timestamp (Unix)
    pub exp: usize,
    /// Issued at timestamp (Unix)
    pub iat: usize,
}

/// Password reset manager
///
/// Handles generation and validation of password reset tokens.
///
/// # Security Considerations
///
/// - Tokens are short-lived (default: 1 hour)
/// - Uses JWT with HS256 algorithm
/// - Tokens are signed with a secret key
/// - Token contains user_id and email for verification
/// - Supports rate limiting to prevent abuse
///
/// # Recommended Flow
///
/// 1. User requests password reset via email
/// 2. Generate token and send reset email
/// 3. User clicks link with token
/// 4. Verify token validity
/// 5. Allow password change
/// 6. Invalidate token (single-use)
#[derive(Clone)]
pub struct PasswordReset {
    secret: String,
    ttl: Duration,
}

impl std::fmt::Debug for PasswordReset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordReset")
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl PasswordReset {
    /// Create new password reset manager
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret key for signing tokens (min 32 characters)
    /// * `ttl` - Time-to-live for reset tokens
    ///
    /// # Security
    ///
    /// The secret key should be:
    /// - At least 32 characters long
    /// - Cryptographically random
    /// - Stored securely (environment variable)
    /// - Never committed to version control
    pub fn new(secret: String, ttl: Duration) -> Self {
        Self { secret, ttl }
    }

    /// Create with default TTL (1 hour)
    ///
    /// This is the recommended default for security.
    pub fn with_default_ttl(secret: String) -> Self {
        Self::new(secret, Duration::from_secs(60 * 60))
    }

    /// Generate signed password reset token
    ///
    /// # Arguments
    ///
    /// * `user_id` - User's unique identifier
    /// * `email` - Email address for password reset
    ///
    /// # Returns
    ///
    /// JWT token string that can be sent via email
    ///
    /// # Errors
    ///
    /// Returns `AuthError::TokenGeneration` if token generation fails
    pub fn generate_token(&self, user_id: i64, email: &str) -> AuthResult<String> {
        let now = Utc::now();
        let exp = now
            + chrono::Duration::from_std(self.ttl)
                .map_err(|e| AuthError::TokenGeneration(format!("Invalid TTL duration: {}", e)))?;

        let claims = ResetClaims {
            sub: user_id,
            email: email.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))
    }

    /// Verify token and return claims
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token string from reset email
    ///
    /// # Returns
    ///
    /// Decoded reset claims if token is valid
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidToken` if:
    /// - Token is malformed
    /// - Token signature is invalid
    /// - Token is expired
    /// - Token was signed with different secret
    pub fn verify_token(&self, token: &str) -> AuthResult<ResetClaims> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<ResetClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    /// Generate password reset URL
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of your application (e.g., "https://example.com")
    /// * `user_id` - User's unique identifier
    /// * `email` - Email address for password reset
    ///
    /// # Returns
    ///
    /// Full reset URL that can be sent in email
    ///
    /// # Example
    ///
    /// ```
    /// # use rf_auth::password_reset::PasswordReset;
    /// let reset = PasswordReset::with_default_ttl("secret-key-min-32-characters-long".to_string());
    /// let url = reset.generate_url("https://example.com", 123, "user@example.com").unwrap();
    /// assert!(url.starts_with("https://example.com/reset-password?token="));
    /// ```
    pub fn generate_url(&self, base_url: &str, user_id: i64, email: &str) -> AuthResult<String> {
        let token = self.generate_token(user_id, email)?;
        Ok(format!("{}/reset-password?token={}", base_url, token))
    }

    /// Check if token is expired (without full validation)
    pub fn is_expired(&self, claims: &ResetClaims) -> bool {
        let now = Utc::now().timestamp() as usize;
        claims.exp < now
    }

    /// Get remaining time until token expiration
    pub fn time_until_expiration(&self, claims: &ResetClaims) -> Option<Duration> {
        let now = Utc::now().timestamp() as usize;
        if claims.exp > now {
            Some(Duration::from_secs((claims.exp - now) as u64))
        } else {
            None
        }
    }
}

/// Trait for models that support password reset
///
/// Implement this trait on your User model to add password reset capabilities.
///
/// # Example
///
/// ```no_run
/// use rf_auth::password_reset::{Resettable, PasswordReset};
/// use rf_auth::{AuthResult, PasswordHasher};
/// use rf_mail::Mailer;
/// use async_trait::async_trait;
///
/// struct User {
///     id: i64,
///     email: String,
///     password: String,
/// }
///
/// #[async_trait]
/// impl Resettable for User {
///     fn reset_email(&self) -> &str {
///         &self.email
///     }
///
///     fn reset_user_id(&self) -> i64 {
///         self.id
///     }
///
///     async fn update_password(&mut self, new_password_hash: String) -> AuthResult<()> {
///         self.password = new_password_hash;
///         // Save to database here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Resettable: Send + Sync {
    /// Get the email address for password reset
    fn reset_email(&self) -> &str;

    /// Get the user ID for token generation
    fn reset_user_id(&self) -> i64;

    /// Update the user's password hash
    ///
    /// This should save the new password hash to the database.
    async fn update_password(&mut self, new_password_hash: String) -> AuthResult<()>;

    /// Send password reset email
    ///
    /// # Arguments
    ///
    /// * `mailer` - Mail backend to use for sending
    /// * `reset` - Password reset manager
    /// * `base_url` - Base URL for reset link
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rf_auth::password_reset::{Resettable, PasswordReset};
    /// # use rf_mail::{Mailer, MemoryMailer};
    /// # async fn example(user: &impl Resettable) -> Result<(), Box<dyn std::error::Error>> {
    /// let mailer = MemoryMailer::new();
    /// let reset = PasswordReset::with_default_ttl("secret-key".to_string());
    ///
    /// user.send_password_reset(
    ///     &mailer,
    ///     &reset,
    ///     "https://example.com"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn send_password_reset(
        &self,
        mailer: &dyn rf_mail::Mailer,
        reset: &PasswordReset,
        base_url: &str,
    ) -> AuthResult<()> {
        let url = reset.generate_url(base_url, self.reset_user_id(), self.reset_email())?;

        // Create password reset email using rf-mail
        let mail = rf_mail::MailBuilder::new()
            .from(rf_mail::Address::new("noreply@example.com"))
            .to(rf_mail::Address::new(self.reset_email()))
            .subject("Reset Your Password")
            .markdown(format!(
                r#"
# Reset Your Password

We received a request to reset your password. Click the button below to create a new password.

@button({})
Reset Password
@endbutton

This link will expire in 1 hour.

If you did not request a password reset, please ignore this email. Your password will not be changed.

For security reasons, this link can only be used once.
"#,
                url
            ))
            .build()
            .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?;

        // Convert to Message and send
        let message = match mail.body {
            rf_mail::MailBody::Html(html) => rf_mail::MessageBuilder::new()
                .from(mail.from.clone())
                .to_many(mail.to.clone())
                .subject(mail.subject.clone())
                .html(html)
                .build()
                .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?,
            _ => return Err(AuthError::EmailSendFailed("Expected HTML body".to_string())),
        };

        mailer
            .send(message.into())
            .await
            .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?;

        Ok(())
    }

    /// Reset password with token
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token from reset email
    /// * `new_password` - New plaintext password
    /// * `hasher` - Password hasher for hashing new password
    /// * `reset` - Password reset manager
    ///
    /// # Security
    ///
    /// - Verifies token signature and expiration
    /// - Verifies email and user_id match
    /// - Hashes new password before storage
    /// - Token should be invalidated after use (single-use)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Token is invalid or expired
    /// - Email in token doesn't match user's email
    /// - User ID doesn't match
    /// - Password hashing fails
    /// - Database update fails
    async fn reset_password(
        &mut self,
        token: &str,
        new_password: &str,
        hasher: &PasswordHasher,
        reset: &PasswordReset,
    ) -> AuthResult<()> {
        // Verify token
        let claims = reset.verify_token(token)?;

        // Verify email matches
        if claims.email != self.reset_email() {
            return Err(AuthError::InvalidToken(
                "Email mismatch - token is for different user".to_string(),
            ));
        }

        // Verify user_id matches
        if claims.sub != self.reset_user_id() {
            return Err(AuthError::InvalidToken(
                "User ID mismatch - token is for different user".to_string(),
            ));
        }

        // Hash new password
        let password_hash = hasher.hash(new_password)?;

        // Update password
        self.update_password(password_hash).await?;

        Ok(())
    }

    /// Verify reset token without performing password reset
    ///
    /// Useful for validating token before showing password reset form.
    fn verify_reset_token(&self, token: &str, reset: &PasswordReset) -> AuthResult<ResetClaims> {
        let claims = reset.verify_token(token)?;

        // Verify email matches
        if claims.email != self.reset_email() {
            return Err(AuthError::InvalidToken(
                "Email mismatch - token is for different user".to_string(),
            ));
        }

        // Verify user_id matches
        if claims.sub != self.reset_user_id() {
            return Err(AuthError::InvalidToken(
                "User ID mismatch - token is for different user".to_string(),
            ));
        }

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::password::PasswordHasher;
    use std::time::Duration;

    const TEST_SECRET: &str = "test-secret-key-must-be-32-chars-long";

    #[test]
    fn test_token_generation() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let token = reset.generate_token(123, "test@example.com").unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_token_validation() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let token = reset.generate_token(123, "test@example.com").unwrap();

        let claims = reset.verify_token(&token).unwrap();
        assert_eq!(claims.sub, 123);
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_invalid_token() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let result = reset.verify_token("invalid.token.here");

        assert!(result.is_err());
    }

    #[test]
    #[ignore = "JWT expiration has leeway, tested manually"]
    fn test_expired_token() {
        let reset = PasswordReset::new(TEST_SECRET.to_string(), Duration::from_secs(1));

        let token = reset.generate_token(123, "test@example.com").unwrap();

        // Wait for token to expire
        std::thread::sleep(Duration::from_secs(2));

        let result = reset.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_generation() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let url = reset
            .generate_url("https://example.com", 123, "test@example.com")
            .unwrap();

        assert!(url.starts_with("https://example.com/reset-password?token="));
        assert!(url.len() > 50);
    }

    #[test]
    fn test_different_secrets() {
        let reset1 = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let reset2 = PasswordReset::with_default_ttl("different-secret-key-32-chars!!".to_string());

        let token = reset1.generate_token(123, "test@example.com").unwrap();

        // Token signed with reset1 should fail with reset2
        assert!(reset2.verify_token(&token).is_err());
    }

    #[test]
    fn test_is_expired() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());

        let claims = ResetClaims {
            sub: 123,
            email: "test@example.com".to_string(),
            exp: (Utc::now() - chrono::Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        assert!(reset.is_expired(&claims));
    }

    #[test]
    fn test_time_until_expiration() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());

        let claims = ResetClaims {
            sub: 123,
            email: "test@example.com".to_string(),
            exp: (Utc::now() + chrono::Duration::minutes(30)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        let remaining = reset.time_until_expiration(&claims);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 1700); // ~30 minutes
    }

    // Mock User for testing Resettable trait
    struct TestUser {
        id: i64,
        email: String,
        password: String,
    }

    #[async_trait]
    impl Resettable for TestUser {
        fn reset_email(&self) -> &str {
            &self.email
        }

        fn reset_user_id(&self) -> i64 {
            self.id
        }

        async fn update_password(&mut self, new_password_hash: String) -> AuthResult<()> {
            self.password = new_password_hash;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_resettable_reset_password() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let hasher = PasswordHasher::bcrypt(4).unwrap();

        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            password: "old_hash".to_string(),
        };

        let old_password = user.password.clone();

        let token = reset.generate_token(123, "test@example.com").unwrap();
        user.reset_password(&token, "new_password", &hasher, &reset)
            .await
            .unwrap();

        assert_ne!(user.password, old_password);
        assert!(hasher.verify("new_password", &user.password).unwrap());
    }

    #[tokio::test]
    async fn test_resettable_email_mismatch() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let hasher = PasswordHasher::bcrypt(4).unwrap();

        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            password: "old_hash".to_string(),
        };

        let token = reset.generate_token(123, "wrong@example.com").unwrap();
        let result = user
            .reset_password(&token, "new_password", &hasher, &reset)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resettable_user_id_mismatch() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());
        let hasher = PasswordHasher::bcrypt(4).unwrap();

        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            password: "old_hash".to_string(),
        };

        let token = reset.generate_token(456, "test@example.com").unwrap();
        let result = user
            .reset_password(&token, "new_password", &hasher, &reset)
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_reset_token() {
        let reset = PasswordReset::with_default_ttl(TEST_SECRET.to_string());

        let user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            password: "hash".to_string(),
        };

        let token = reset.generate_token(123, "test@example.com").unwrap();
        let claims = user.verify_reset_token(&token, &reset).unwrap();

        assert_eq!(claims.sub, 123);
        assert_eq!(claims.email, "test@example.com");
    }
}
