//! Remember Me cookie and token management

use crate::error::{AuthError, AuthResult};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Claims for remember me tokens
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberClaims {
    /// User ID (subject)
    pub sub: i64,
    /// Expiration timestamp (Unix)
    pub exp: usize,
    /// Issued at timestamp (Unix)
    pub iat: usize,
    /// Token ID for rotation tracking
    pub jti: String,
}

/// Remember Me cookie manager
///
/// Handles generation and validation of remember me tokens and cookies.
///
/// # Security Features
///
/// - JWT-based tokens with HS256 signature
/// - HTTP-only cookies (not accessible via JavaScript)
/// - Secure flag (transmitted only over HTTPS)
/// - SameSite=Strict (CSRF protection)
/// - Token rotation on each use (optional)
/// - Long expiration but revocable
///
/// # Token Rotation
///
/// For enhanced security, tokens can be rotated on each use:
/// 1. User logs in with remember me
/// 2. Token is created and stored in cookie
/// 3. On next visit, token is validated
/// 4. New token is generated and old one invalidated
/// 5. Repeat for each visit
///
/// This limits the window of opportunity if a token is compromised.
#[derive(Clone)]
pub struct RememberMe {
    secret: String,
    ttl: Duration,
}

impl std::fmt::Debug for RememberMe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RememberMe")
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl RememberMe {
    /// Cookie name for remember me token
    pub const COOKIE_NAME: &'static str = "remember_token";

    /// Create new remember me manager
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret key for signing tokens (min 32 characters)
    /// * `ttl` - Time-to-live for remember me tokens
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

    /// Create with default TTL (30 days)
    ///
    /// This is a reasonable default for most applications.
    pub fn with_default_ttl(secret: String) -> Self {
        Self::new(secret, Duration::from_secs(30 * 24 * 60 * 60))
    }

    /// Generate remember me token
    ///
    /// # Arguments
    ///
    /// * `user_id` - User's unique identifier
    ///
    /// # Returns
    ///
    /// JWT token string that can be stored in cookie
    ///
    /// # Errors
    ///
    /// Returns `AuthError::TokenGeneration` if token generation fails
    pub fn generate_token(&self, user_id: i64) -> AuthResult<String> {
        let now = Utc::now();
        let exp = now
            + chrono::Duration::from_std(self.ttl)
                .map_err(|e| AuthError::TokenGeneration(format!("Invalid TTL duration: {}", e)))?;

        // Generate unique token ID for rotation
        let jti = uuid::Uuid::new_v4().to_string();

        let claims = RememberClaims {
            sub: user_id,
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti,
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AuthError::TokenGeneration(e.to_string()))
    }

    /// Verify token and return user ID
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token string from remember me cookie
    ///
    /// # Returns
    ///
    /// User ID if token is valid
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidToken` if:
    /// - Token is malformed
    /// - Token signature is invalid
    /// - Token is expired
    /// - Token was signed with different secret
    pub fn verify_token(&self, token: &str) -> AuthResult<i64> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 0; // No clock skew tolerance for strict expiration checking

        let claims = decode::<RememberClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(claims.sub)
    }

    /// Verify token and return full claims
    ///
    /// Useful when you need access to token metadata (e.g., for rotation).
    pub fn verify_token_full(&self, token: &str) -> AuthResult<RememberClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 0; // No clock skew tolerance for strict expiration checking

        decode::<RememberClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    /// Create remember me cookie
    ///
    /// # Arguments
    ///
    /// * `user_id` - User's unique identifier
    ///
    /// # Returns
    ///
    /// Cookie with secure settings that can be added to HTTP response
    ///
    /// # Security
    ///
    /// The cookie has the following security attributes:
    /// - `HttpOnly`: Cannot be accessed via JavaScript (XSS protection)
    /// - `Secure`: Only transmitted over HTTPS
    /// - `SameSite=Strict`: Only sent for same-site requests (CSRF protection)
    /// - Long expiration matching token TTL
    ///
    /// # Example
    ///
    /// ```
    /// # use rf_auth::remember_me::RememberMe;
    /// let remember = RememberMe::with_default_ttl("secret-key-min-32-characters-long".to_string());
    /// let cookie = remember.create_cookie(123).unwrap();
    /// // Cookie is an HTTP header value containing Set-Cookie header string
    /// let cookie_str = cookie.to_str().unwrap();
    /// assert!(cookie_str.contains("remember_token="));
    /// assert!(cookie_str.contains("HttpOnly"));
    /// assert!(cookie_str.contains("Secure"));
    /// ```
    pub fn create_cookie(&self, user_id: i64) -> AuthResult<axum::http::HeaderValue> {
        let token = self.generate_token(user_id)?;

        // Build cookie string manually for maximum control
        let max_age = self.ttl.as_secs();
        let cookie_str = format!(
            "{}={}; HttpOnly; Secure; SameSite=Strict; Max-Age={}; Path=/",
            Self::COOKIE_NAME,
            token,
            max_age
        );

        axum::http::HeaderValue::from_str(&cookie_str)
            .map_err(|e| AuthError::TokenGeneration(format!("Invalid cookie value: {}", e)))
    }

    /// Create cookie for deletion
    ///
    /// Returns a cookie that clears the remember me cookie.
    ///
    /// # Example
    ///
    /// ```
    /// # use rf_auth::remember_me::RememberMe;
    /// let remember = RememberMe::with_default_ttl("secret-key-min-32-characters-long".to_string());
    /// let delete_cookie = remember.delete_cookie();
    /// // Add to response to logout user
    /// ```
    pub fn delete_cookie(&self) -> axum::http::HeaderValue {
        let cookie_str = format!(
            "{}=; HttpOnly; Secure; SameSite=Strict; Max-Age=0; Path=/",
            Self::COOKIE_NAME
        );

        axum::http::HeaderValue::from_str(&cookie_str)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static(""))
    }

    /// Rotate token (generate new token for same user)
    ///
    /// # Arguments
    ///
    /// * `old_token` - Current token to rotate
    ///
    /// # Returns
    ///
    /// New token for the same user
    ///
    /// # Security
    ///
    /// Token rotation is a security best practice that:
    /// - Limits the lifetime of any single token
    /// - Reduces impact of token compromise
    /// - Enables detection of stolen tokens (if multiple active)
    ///
    /// # Usage
    ///
    /// Call this on each authenticated request to rotate the token.
    /// Old token should be invalidated (if using token storage).
    pub fn rotate_token(&self, old_token: &str) -> AuthResult<String> {
        let claims = self.verify_token_full(old_token)?;
        self.generate_token(claims.sub)
    }

    /// Check if token is expired (without full validation)
    pub fn is_expired(&self, claims: &RememberClaims) -> bool {
        let now = Utc::now().timestamp() as usize;
        claims.exp < now
    }

    /// Get remaining time until token expiration
    pub fn time_until_expiration(&self, claims: &RememberClaims) -> Option<Duration> {
        let now = Utc::now().timestamp() as usize;
        if claims.exp > now {
            Some(Duration::from_secs((claims.exp - now) as u64))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-must-be-32-chars-long";

    #[test]
    fn test_token_generation() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let token = remember.generate_token(123).unwrap();

        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_token_validation() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let token = remember.generate_token(123).unwrap();

        let user_id = remember.verify_token(&token).unwrap();
        assert_eq!(user_id, 123);
    }

    #[test]
    fn test_token_full_validation() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let token = remember.generate_token(123).unwrap();

        let claims = remember.verify_token_full(&token).unwrap();
        assert_eq!(claims.sub, 123);
        assert!(!claims.jti.is_empty());
    }

    #[test]
    fn test_invalid_token() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let result = remember.verify_token("invalid.token.here");

        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let remember = RememberMe::new(TEST_SECRET.to_string(), Duration::from_secs(1));

        let token = remember.generate_token(123).unwrap();

        // Wait for token to expire
        std::thread::sleep(Duration::from_secs(2));

        let result = remember.verify_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_cookie_creation() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let cookie = remember.create_cookie(123).unwrap();

        let cookie_str = cookie.to_str().unwrap();
        assert!(cookie_str.contains("remember_token="));
        assert!(cookie_str.contains("HttpOnly"));
        assert!(cookie_str.contains("Secure"));
        assert!(cookie_str.contains("SameSite=Strict"));
    }

    #[test]
    fn test_delete_cookie() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let cookie = remember.delete_cookie();

        let cookie_str = cookie.to_str().unwrap();
        assert!(cookie_str.contains("remember_token="));
        assert!(cookie_str.contains("Max-Age=0"));
    }

    #[test]
    fn test_token_rotation() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let old_token = remember.generate_token(123).unwrap();

        let new_token = remember.rotate_token(&old_token).unwrap();

        // Both tokens should be for same user
        assert_eq!(remember.verify_token(&old_token).unwrap(), 123);
        assert_eq!(remember.verify_token(&new_token).unwrap(), 123);

        // But tokens should be different
        assert_ne!(old_token, new_token);

        // And have different JTIs
        let old_claims = remember.verify_token_full(&old_token).unwrap();
        let new_claims = remember.verify_token_full(&new_token).unwrap();
        assert_ne!(old_claims.jti, new_claims.jti);
    }

    #[test]
    fn test_different_secrets() {
        let remember1 = RememberMe::with_default_ttl(TEST_SECRET.to_string());
        let remember2 = RememberMe::with_default_ttl("different-secret-key-32-chars!!".to_string());

        let token = remember1.generate_token(123).unwrap();

        // Token signed with remember1 should fail with remember2
        assert!(remember2.verify_token(&token).is_err());
    }

    #[test]
    fn test_is_expired() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

        let claims = RememberClaims {
            sub: 123,
            exp: (Utc::now() - chrono::Duration::hours(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
            jti: "test".to_string(),
        };

        assert!(remember.is_expired(&claims));
    }

    #[test]
    fn test_time_until_expiration() {
        let remember = RememberMe::with_default_ttl(TEST_SECRET.to_string());

        let claims = RememberClaims {
            sub: 123,
            exp: (Utc::now() + chrono::Duration::days(1)).timestamp() as usize,
            iat: Utc::now().timestamp() as usize,
            jti: "test".to_string(),
        };

        let remaining = remember.time_until_expiration(&claims);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 86000); // ~1 day
    }
}
