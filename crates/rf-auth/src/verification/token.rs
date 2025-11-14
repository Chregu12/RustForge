//! Email verification token generation and validation

use crate::error::{AuthError, AuthResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Claims for email verification tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationClaims {
    /// User ID (subject)
    pub sub: i64,
    /// Email address being verified
    pub email: String,
    /// Expiration timestamp (Unix)
    pub exp: usize,
    /// Issued at timestamp (Unix)
    pub iat: usize,
}

/// Email verification manager
///
/// Handles generation and validation of email verification tokens.
///
/// # Security
///
/// - Uses JWT with HS256 algorithm
/// - Tokens are signed with a secret key
/// - Tokens expire after configured duration (default: 24h)
/// - Token contains user_id and email for verification
#[derive(Clone)]
pub struct EmailVerification {
    secret: String,
    ttl: Duration,
}

impl std::fmt::Debug for EmailVerification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailVerification")
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl EmailVerification {
    /// Create new email verification manager
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret key for signing tokens (min 32 characters)
    /// * `ttl` - Time-to-live for verification tokens
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

    /// Create with default TTL (24 hours)
    pub fn with_default_ttl(secret: String) -> Self {
        Self::new(secret, Duration::from_secs(24 * 60 * 60))
    }

    /// Generate signed verification token
    ///
    /// # Arguments
    ///
    /// * `user_id` - User's unique identifier
    /// * `email` - Email address to verify
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
        let exp = now + chrono::Duration::from_std(self.ttl).map_err(|e| {
            AuthError::TokenGeneration(format!("Invalid TTL duration: {}", e))
        })?;

        let claims = VerificationClaims {
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
    /// * `token` - JWT token string from verification email
    ///
    /// # Returns
    ///
    /// Decoded verification claims if token is valid
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidToken` if:
    /// - Token is malformed
    /// - Token signature is invalid
    /// - Token is expired
    /// - Token was signed with different secret
    pub fn verify_token(&self, token: &str) -> AuthResult<VerificationClaims> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<VerificationClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    /// Generate verification URL
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of your application (e.g., "https://example.com")
    /// * `user_id` - User's unique identifier
    /// * `email` - Email address to verify
    ///
    /// # Returns
    ///
    /// Full verification URL that can be sent in email
    ///
    /// # Example
    ///
    /// ```
    /// # use rf_auth::verification::EmailVerification;
    /// # use std::time::Duration;
    /// let verification = EmailVerification::with_default_ttl("secret-key-min-32-characters-long".to_string());
    /// let url = verification.generate_url("https://example.com", 123, "user@example.com").unwrap();
    /// assert!(url.starts_with("https://example.com/verify-email?token="));
    /// ```
    pub fn generate_url(&self, base_url: &str, user_id: i64, email: &str) -> AuthResult<String> {
        let token = self.generate_token(user_id, email)?;
        Ok(format!("{}/verify-email?token={}", base_url, token))
    }

    /// Check if token is expired (without full validation)
    pub fn is_expired(&self, claims: &VerificationClaims) -> bool {
        let now = Utc::now().timestamp() as usize;
        claims.exp < now
    }

    /// Get remaining time until token expiration
    pub fn time_until_expiration(&self, claims: &VerificationClaims) -> Option<Duration> {
        let now = Utc::now().timestamp() as usize;
        if claims.exp > now {
            Some(Duration::from_secs((claims.exp - now) as u64))
        } else {
            None
        }
    }
}

/// Trait for models that support email verification
///
/// Implement this trait on your User model to add email verification capabilities.
///
/// # Example
///
/// ```no_run
/// use rf_auth::verification::{Verifiable, EmailVerification};
/// use rf_auth::AuthResult;
/// use rf_mail::Mailer;
/// use async_trait::async_trait;
/// use chrono::{DateTime, Utc};
///
/// struct User {
///     id: i64,
///     email: String,
///     email_verified_at: Option<DateTime<Utc>>,
/// }
///
/// #[async_trait]
/// impl Verifiable for User {
///     fn verification_email(&self) -> &str {
///         &self.email
///     }
///
///     fn verification_user_id(&self) -> i64 {
///         self.id
///     }
///
///     fn is_verified(&self) -> bool {
///         self.email_verified_at.is_some()
///     }
///
///     async fn mark_verified(&mut self) -> AuthResult<()> {
///         self.email_verified_at = Some(Utc::now());
///         // Save to database here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Verifiable: Send + Sync {
    /// Get the email address to verify
    fn verification_email(&self) -> &str;

    /// Get the user ID for token generation
    fn verification_user_id(&self) -> i64;

    /// Check if email is already verified
    fn is_verified(&self) -> bool;

    /// Mark email as verified
    ///
    /// This should update the database with the current timestamp.
    async fn mark_verified(&mut self) -> AuthResult<()>;

    /// Send verification email
    ///
    /// # Arguments
    ///
    /// * `mailer` - Mail backend to use for sending
    /// * `verification` - Email verification manager
    /// * `base_url` - Base URL for verification link
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rf_auth::verification::{Verifiable, EmailVerification};
    /// # use rf_mail::{Mailer, MemoryMailer};
    /// # async fn example(user: &impl Verifiable) -> Result<(), Box<dyn std::error::Error>> {
    /// let mailer = MemoryMailer::new();
    /// let verification = EmailVerification::with_default_ttl("secret-key".to_string());
    ///
    /// user.send_verification_email(
    ///     &mailer,
    ///     &verification,
    ///     "https://example.com"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn send_verification_email(
        &self,
        mailer: &dyn rf_mail::Mailer,
        verification: &EmailVerification,
        base_url: &str,
    ) -> AuthResult<()> {
        let url = verification.generate_url(
            base_url,
            self.verification_user_id(),
            self.verification_email(),
        )?;

        // Create verification email using rf-mail
        let mail = rf_mail::MailBuilder::new()
            .from(rf_mail::Address::new("noreply@example.com"))
            .to(rf_mail::Address::new(self.verification_email()))
            .subject("Verify Your Email Address")
            .markdown(&format!(
                r#"
# Verify Your Email Address

Thank you for signing up! Please click the button below to verify your email address.

@button({})
Verify Email
@endbutton

This link will expire in 24 hours.

If you did not create an account, please ignore this email.
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
            _ => {
                return Err(AuthError::EmailSendFailed(
                    "Expected HTML body".to_string(),
                ))
            }
        };

        mailer
            .send(&message)
            .await
            .map_err(|e| AuthError::EmailSendFailed(e.to_string()))?;

        Ok(())
    }

    /// Verify email with token
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token from verification email
    /// * `verification` - Email verification manager
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Token is invalid or expired
    /// - Email in token doesn't match user's email
    /// - Database update fails
    async fn verify_email(
        &mut self,
        token: &str,
        verification: &EmailVerification,
    ) -> AuthResult<()> {
        let claims = verification.verify_token(token)?;

        // Verify email matches
        if claims.email != self.verification_email() {
            return Err(AuthError::InvalidToken(
                "Email mismatch - token is for different user".to_string(),
            ));
        }

        // Verify user_id matches
        if claims.sub != self.verification_user_id() {
            return Err(AuthError::InvalidToken(
                "User ID mismatch - token is for different user".to_string(),
            ));
        }

        // Mark as verified
        self.mark_verified().await?;

        Ok(())
    }

    /// Resend verification email
    ///
    /// Same as `send_verification_email` but checks if already verified first.
    async fn resend_verification_email(
        &self,
        mailer: &dyn rf_mail::Mailer,
        verification: &EmailVerification,
        base_url: &str,
    ) -> AuthResult<()> {
        if self.is_verified() {
            return Err(AuthError::AlreadyVerified);
        }

        self.send_verification_email(mailer, verification, base_url)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    const TEST_SECRET: &str = "test-secret-key-must-be-32-chars-long";

    #[test]
    fn test_token_generation() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let token = verification
            .generate_token(123, "test@example.com")
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_token_validation() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let token = verification
            .generate_token(123, "test@example.com")
            .unwrap();

        let claims = verification.verify_token(&token).unwrap();
        assert_eq!(claims.sub, 123);
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_invalid_token() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let result = verification.verify_token("invalid.token.here");

        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let verification =
            EmailVerification::new(TEST_SECRET.to_string(), Duration::from_secs(1));

        let token = verification
            .generate_token(123, "test@example.com")
            .unwrap();

        // Wait for token to expire
        std::thread::sleep(Duration::from_secs(2));

        let result = verification.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_generation() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let url = verification
            .generate_url("https://example.com", 123, "test@example.com")
            .unwrap();

        assert!(url.starts_with("https://example.com/verify-email?token="));
        assert!(url.len() > 50);
    }

    #[test]
    fn test_different_secrets() {
        let verification1 = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let verification2 =
            EmailVerification::with_default_ttl("different-secret-key-32-chars!!".to_string());

        let token = verification1
            .generate_token(123, "test@example.com")
            .unwrap();

        // Token signed with verification1 should fail with verification2
        assert!(verification2.verify_token(&token).is_err());
    }

    #[test]
    fn test_is_expired() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());

        let claims = VerificationClaims {
            sub: 123,
            email: "test@example.com".to_string(),
            exp: (Utc::now() - chrono::Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        assert!(verification.is_expired(&claims));
    }

    #[test]
    fn test_time_until_expiration() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());

        let claims = VerificationClaims {
            sub: 123,
            email: "test@example.com".to_string(),
            exp: (Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
        };

        let remaining = verification.time_until_expiration(&claims);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 3500); // ~1 hour
    }

    // Mock User for testing Verifiable trait
    struct TestUser {
        id: i64,
        email: String,
        email_verified_at: Option<DateTime<Utc>>,
    }

    #[async_trait]
    impl Verifiable for TestUser {
        fn verification_email(&self) -> &str {
            &self.email
        }

        fn verification_user_id(&self) -> i64 {
            self.id
        }

        fn is_verified(&self) -> bool {
            self.email_verified_at.is_some()
        }

        async fn mark_verified(&mut self) -> AuthResult<()> {
            self.email_verified_at = Some(Utc::now());
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_verifiable_verify_email() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            email_verified_at: None,
        };

        assert!(!user.is_verified());

        let token = verification.generate_token(123, "test@example.com").unwrap();
        user.verify_email(&token, &verification).await.unwrap();

        assert!(user.is_verified());
    }

    #[tokio::test]
    async fn test_verifiable_email_mismatch() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            email_verified_at: None,
        };

        let token = verification
            .generate_token(123, "wrong@example.com")
            .unwrap();
        let result = user.verify_email(&token, &verification).await;

        assert!(result.is_err());
        assert!(!user.is_verified());
    }

    #[tokio::test]
    async fn test_verifiable_user_id_mismatch() {
        let verification = EmailVerification::with_default_ttl(TEST_SECRET.to_string());
        let mut user = TestUser {
            id: 123,
            email: "test@example.com".to_string(),
            email_verified_at: None,
        };

        let token = verification
            .generate_token(456, "test@example.com")
            .unwrap();
        let result = user.verify_email(&token, &verification).await;

        assert!(result.is_err());
        assert!(!user.is_verified());
    }
}
