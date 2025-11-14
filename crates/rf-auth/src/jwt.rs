//! JWT token generation and validation
//!
//! Provides JWT-based authentication with access and refresh tokens.

use crate::error::{AuthError, AuthResult};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims structure
///
/// Contains standard JWT claims plus custom user data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// Subject (user identifier, usually email)
    pub sub: String,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// JWT ID (unique identifier for token)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,

    /// Custom: User ID
    pub user_id: i32,

    /// Custom: User roles
    pub roles: Vec<String>,
}

impl Claims {
    /// Create new claims with expiry
    ///
    /// # Arguments
    ///
    /// * `user_id` - User's unique identifier
    /// * `email` - User's email address (used as subject)
    /// * `roles` - User's roles (for authorization)
    /// * `expiry_hours` - Token expiry in hours
    pub fn new(user_id: i32, email: String, roles: Vec<String>, expiry_hours: u64) -> Self {
        let now = Utc::now();
        Self {
            sub: email,
            exp: (now + Duration::hours(expiry_hours as i64)).timestamp(),
            iat: now.timestamp(),
            jti: Some(Uuid::new_v4().to_string()),
            user_id,
            roles,
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        self.exp < Utc::now().timestamp()
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the specified roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// JWT token manager
///
/// Handles JWT token generation and validation.
///
/// # Examples
///
/// ```
/// use rf_auth::{JwtManager, Claims};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create JWT manager with secret
/// let jwt = JwtManager::new("your-secret-key-min-32-characters")?;
///
/// // Create claims
/// let claims = Claims::new(
///     123,
///     "user@example.com".to_string(),
///     vec!["user".to_string()],
///     24, // 24 hours
/// );
///
/// // Generate token
/// let token = jwt.generate_token(&claims)?;
///
/// // Validate and decode token
/// let decoded = jwt.validate_token(&token)?;
/// assert_eq!(decoded.user_id, 123);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("validation", &self.validation)
            .finish_non_exhaustive()
    }
}

impl JwtManager {
    /// Create a new JWT manager
    ///
    /// # Arguments
    ///
    /// * `secret` - Secret key for signing tokens (min 32 characters)
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidSecret` if secret is less than 32 characters
    pub fn new(secret: &str) -> AuthResult<Self> {
        if secret.len() < 32 {
            return Err(AuthError::InvalidSecret);
        }

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::new(Algorithm::HS256);

        Ok(Self {
            encoding_key,
            decoding_key,
            validation,
        })
    }

    /// Generate an access token
    ///
    /// # Arguments
    ///
    /// * `claims` - Claims to encode in the token
    ///
    /// # Returns
    ///
    /// JWT token string
    ///
    /// # Errors
    ///
    /// Returns `AuthError::JwtError` if token generation fails
    pub fn generate_token(&self, claims: &Claims) -> AuthResult<String> {
        let header = Header::new(Algorithm::HS256);
        let token = encode(&header, claims, &self.encoding_key)?;
        Ok(token)
    }

    /// Generate a refresh token
    ///
    /// Refresh tokens have longer expiry (7 days by default).
    ///
    /// # Arguments
    ///
    /// * `claims` - Claims to encode (expiry will be extended)
    ///
    /// # Returns
    ///
    /// JWT refresh token string
    pub fn generate_refresh_token(&self, claims: &Claims) -> AuthResult<String> {
        let mut refresh_claims = claims.clone();
        refresh_claims.exp = (Utc::now() + Duration::days(7)).timestamp();
        refresh_claims.jti = Some(Uuid::new_v4().to_string()); // New JTI for refresh token

        self.generate_token(&refresh_claims)
    }

    /// Validate and decode a token
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token string to validate
    ///
    /// # Returns
    ///
    /// Decoded claims if token is valid
    ///
    /// # Errors
    ///
    /// Returns `AuthError::JwtError` if token is invalid or expired
    pub fn validate_token(&self, token: &str) -> AuthResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)?;
        Ok(token_data.claims)
    }

    /// Validate a refresh token
    ///
    /// # Arguments
    ///
    /// * `token` - Refresh token string to validate
    ///
    /// # Returns
    ///
    /// Decoded claims if refresh token is valid
    ///
    /// # Errors
    ///
    /// Returns `AuthError::JwtError` if token is invalid or expired
    pub fn validate_refresh_token(&self, token: &str) -> AuthResult<Claims> {
        // Same validation as regular token
        self.validate_token(token)
    }

    /// Check if token is expired without full validation
    ///
    /// # Arguments
    ///
    /// * `claims` - Claims to check
    ///
    /// # Returns
    ///
    /// `true` if token is expired
    pub fn is_expired(&self, claims: &Claims) -> bool {
        claims.is_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-min-32-characters-long";

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new(
            123,
            "user@example.com".to_string(),
            vec!["user".to_string(), "admin".to_string()],
            24,
        );

        assert_eq!(claims.user_id, 123);
        assert_eq!(claims.sub, "user@example.com");
        assert_eq!(claims.roles.len(), 2);
        assert!(claims.jti.is_some());
    }

    #[test]
    fn test_claims_expiry() {
        let mut claims = Claims::new(1, "test@example.com".into(), vec![], 1);

        // Not expired yet
        assert!(!claims.is_expired());

        // Manually set expiry to past
        claims.exp = (Utc::now() - Duration::hours(1)).timestamp();
        assert!(claims.is_expired());
    }

    #[test]
    fn test_claims_role_check() {
        let claims = Claims::new(
            1,
            "test@example.com".into(),
            vec!["user".into(), "moderator".into()],
            1,
        );

        assert!(claims.has_role("user"));
        assert!(claims.has_role("moderator"));
        assert!(!claims.has_role("admin"));

        assert!(claims.has_any_role(&["user", "admin"]));
        assert!(!claims.has_any_role(&["admin", "superuser"]));

        assert!(claims.has_all_roles(&["user", "moderator"]));
        assert!(!claims.has_all_roles(&["user", "admin"]));
    }

    #[test]
    fn test_jwt_manager_creation() {
        assert!(JwtManager::new(TEST_SECRET).is_ok());
        assert!(JwtManager::new("short").is_err());
    }

    #[test]
    fn test_token_generation() {
        let jwt = JwtManager::new(TEST_SECRET).unwrap();
        let claims = Claims::new(1, "test@example.com".into(), vec!["user".into()], 1);

        let token = jwt.generate_token(&claims).unwrap();
        assert!(!token.is_empty());
        assert!(token.contains('.'));
    }

    #[test]
    fn test_token_validation() {
        let jwt = JwtManager::new(TEST_SECRET).unwrap();
        let claims = Claims::new(123, "test@example.com".into(), vec!["user".into()], 1);

        let token = jwt.generate_token(&claims).unwrap();
        let decoded = jwt.validate_token(&token).unwrap();

        assert_eq!(decoded.user_id, 123);
        assert_eq!(decoded.sub, "test@example.com");
        assert_eq!(decoded.roles, vec!["user"]);
    }

    #[test]
    fn test_invalid_token() {
        let jwt = JwtManager::new(TEST_SECRET).unwrap();
        let result = jwt.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let jwt = JwtManager::new(TEST_SECRET).unwrap();
        let mut claims = Claims::new(1, "test@example.com".into(), vec![], 1);

        // Set expiry to past
        claims.exp = (Utc::now() - Duration::hours(1)).timestamp();

        let token = jwt.generate_token(&claims).unwrap();
        let result = jwt.validate_token(&token);

        // Should fail because token is expired
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_token() {
        let jwt = JwtManager::new(TEST_SECRET).unwrap();
        let claims = Claims::new(1, "test@example.com".into(), vec!["user".into()], 1);

        let refresh_token = jwt.generate_refresh_token(&claims).unwrap();
        let decoded = jwt.validate_refresh_token(&refresh_token).unwrap();

        assert_eq!(decoded.user_id, 1);
        // Refresh token JTI should be different
        assert_ne!(decoded.jti, claims.jti);
    }

    #[test]
    fn test_different_secrets() {
        let jwt1 = JwtManager::new(TEST_SECRET).unwrap();
        let jwt2 = JwtManager::new("different-secret-key-min-32-chars").unwrap();

        let claims = Claims::new(1, "test@example.com".into(), vec![], 1);
        let token = jwt1.generate_token(&claims).unwrap();

        // Token signed with jwt1 should fail validation with jwt2
        assert!(jwt2.validate_token(&token).is_err());
    }
}
