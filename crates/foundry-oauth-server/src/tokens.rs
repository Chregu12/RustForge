//! # OAuth2 Token Management
//!
//! This module provides JWT token generation, validation, and introspection
//! according to OAuth2 and JWT specifications (RFC 6749, RFC 7519, RFC 7662).
//!
//! ## Token Types
//!
//! - **Access Tokens**: Short-lived JWT tokens for API access, containing user and scope information
//! - **Refresh Tokens**: Long-lived opaque tokens for obtaining new access tokens without re-authentication
//!
//! ## Security
//!
//! All tokens are cryptographically signed using the configured JWT secret. Access tokens use
//! JWT with HS256 signing algorithm by default. Refresh tokens are random opaque strings
//! generated using a cryptographically secure random number generator.
//!
//! ## Example
//!
//! ```rust
//! use foundry_oauth_server::tokens::TokenGenerator;
//! use uuid::Uuid;
//!
//! let generator = TokenGenerator::new(
//!     "your-secret-key-min-256-bits".to_string(),
//!     "https://your-issuer.com".to_string()
//! );
//!
//! let token = generator.generate_access_token(
//!     Uuid::new_v4(),
//!     Some(Uuid::new_v4()),
//!     vec!["api:read".to_string()],
//!     3600,
//! )?;
//!
//! println!("Access Token: {}", token.token);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::errors::{OAuth2Error, OAuth2Result};
use crate::models::{AccessToken, RefreshToken};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Token Claims (RFC 7519)
///
/// Standard JWT claims plus OAuth2-specific claims for access tokens.
/// These claims are embedded in the JWT payload and cryptographically signed.
///
/// # Standard Claims
///
/// - `sub` (subject): The principal (user or client) the token represents
/// - `iss` (issuer): The authorization server that issued the token
/// - `aud` (audience): Optional intended recipients of the token
/// - `exp` (expiration): Unix timestamp when the token expires
/// - `iat` (issued at): Unix timestamp when the token was issued
/// - `nbf` (not before): Unix timestamp before which the token is not valid
/// - `jti` (JWT ID): Unique identifier for the token
///
/// # OAuth2 Custom Claims
///
/// - `client_id`: The OAuth2 client that requested the token
/// - `user_id`: Optional user identifier (absent for client credentials grants)
/// - `scopes`: List of granted OAuth2 scopes
/// - `token_type`: Type of token (typically "access_token")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject (user ID or client ID)
    pub sub: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: Option<Vec<String>>,

    /// Expiration time (Unix timestamp)
    pub exp: i64,

    /// Issued at (Unix timestamp)
    pub iat: i64,

    /// Not before (Unix timestamp)
    pub nbf: i64,

    /// JWT ID
    pub jti: String,

    /// Client ID
    pub client_id: String,

    /// User ID (optional, not present for client credentials)
    pub user_id: Option<String>,

    /// Scopes
    pub scopes: Vec<String>,

    /// Token type (access_token, refresh_token)
    pub token_type: String,
}

/// Token Generator
///
/// Generates cryptographically signed JWT access tokens and opaque refresh tokens.
/// Thread-safe and can be cloned for use across multiple tasks.
///
/// # Security
///
/// The JWT secret must have at least 256 bits of entropy for secure signing.
/// Tokens are signed using HS256 (HMAC-SHA256) algorithm.
///
/// # Example
///
/// ```rust
/// use foundry_oauth_server::tokens::TokenGenerator;
/// use uuid::Uuid;
///
/// let generator = TokenGenerator::new(
///     "your-256-bit-secret-key".to_string(),
///     "https://auth.example.com".to_string()
/// );
///
/// let access_token = generator.generate_access_token(
///     Uuid::new_v4(),  // client_id
///     Some(Uuid::new_v4()),  // user_id
///     vec!["read".to_string(), "write".to_string()],
///     3600,  // 1 hour lifetime
/// )?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct TokenGenerator {
    jwt_secret: String,
    issuer: String,
}

impl TokenGenerator {
    /// Create a new token generator
    ///
    /// # Arguments
    ///
    /// * `jwt_secret` - Secret key for JWT signing (min 256 bits recommended)
    /// * `issuer` - Issuer identifier (typically your authorization server URL)
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::TokenGenerator;
    ///
    /// let generator = TokenGenerator::new(
    ///     "your-secret-key".to_string(),
    ///     "https://auth.example.com".to_string()
    /// );
    /// ```
    pub fn new(jwt_secret: String, issuer: String) -> Self {
        Self { jwt_secret, issuer }
    }

    /// Generate a JWT access token
    ///
    /// Creates a signed JWT access token with the specified client, user, scopes, and lifetime.
    /// The token includes standard OAuth2 claims (sub, iss, aud, exp, iat, nbf, jti) plus
    /// custom claims for client_id, user_id, scopes, and token_type.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The OAuth2 client requesting the token
    /// * `user_id` - Optional user ID (None for client credentials grant)
    /// * `scopes` - List of authorized scopes (must be pre-validated)
    /// * `lifetime` - Token lifetime in seconds
    ///
    /// # Returns
    ///
    /// Returns an `AccessToken` containing the signed JWT and metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::TokenGenerator;
    /// use uuid::Uuid;
    ///
    /// let generator = TokenGenerator::new(
    ///     "secret".to_string(),
    ///     "issuer".to_string()
    /// );
    ///
    /// let token = generator.generate_access_token(
    ///     Uuid::new_v4(),
    ///     Some(Uuid::new_v4()),
    ///     vec!["users:read".to_string()],
    ///     3600,
    /// )?;
    ///
    /// println!("Token: {}", token.token);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Security
    ///
    /// The token is signed using the configured JWT secret. Ensure the secret
    /// has at least 256 bits of entropy. Tokens should be transmitted over HTTPS only.
    ///
    /// # Errors
    ///
    /// Returns `OAuth2Error::JwtError` if token generation fails.
    pub fn generate_access_token(
        &self,
        client_id: Uuid,
        user_id: Option<Uuid>,
        scopes: Vec<String>,
        lifetime: i64,
    ) -> OAuth2Result<AccessToken> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(lifetime);
        let token_id = Uuid::new_v4();

        let claims = TokenClaims {
            sub: user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| client_id.to_string()),
            iss: self.issuer.clone(),
            aud: None,
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: token_id.to_string(),
            client_id: client_id.to_string(),
            user_id: user_id.map(|id| id.to_string()),
            scopes: scopes.clone(),
            token_type: "access_token".to_string(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        Ok(AccessToken {
            id: token_id,
            client_id,
            user_id,
            token,
            scopes,
            revoked: false,
            expires_at,
            created_at: now,
        })
    }

    /// Generate an opaque refresh token
    ///
    /// Creates a cryptographically secure random refresh token that can be exchanged
    /// for a new access token. Refresh tokens are opaque (not JWTs) and must be
    /// stored server-side for validation.
    ///
    /// # Arguments
    ///
    /// * `access_token_id` - The ID of the access token this refresh token is associated with
    /// * `lifetime` - Refresh token lifetime in seconds
    ///
    /// # Returns
    ///
    /// Returns a `RefreshToken` containing a random opaque token string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::TokenGenerator;
    /// use uuid::Uuid;
    ///
    /// let generator = TokenGenerator::new("secret".to_string(), "issuer".to_string());
    ///
    /// let refresh_token = generator.generate_refresh_token(
    ///     Uuid::new_v4(),
    ///     2592000,  // 30 days
    /// )?;
    ///
    /// println!("Refresh Token: {}", refresh_token.token);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Security
    ///
    /// Refresh tokens are generated using a cryptographically secure random number
    /// generator and base64 encoded. Store refresh tokens securely server-side.
    ///
    /// # Errors
    ///
    /// This function is infallible and always returns Ok.
    pub fn generate_refresh_token(
        &self,
        access_token_id: Uuid,
        lifetime: i64,
    ) -> OAuth2Result<RefreshToken> {
        let now = Utc::now();
        let expires_at = now + Duration::seconds(lifetime);

        // Generate random token string
        let token = self.generate_random_token();

        Ok(RefreshToken {
            id: Uuid::new_v4(),
            access_token_id,
            token,
            revoked: false,
            expires_at,
            created_at: now,
        })
    }

    fn generate_random_token(&self) -> String {
        use rand::Rng;
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let random_bytes: Vec<u8> = rand::thread_rng()
            .sample_iter(rand::distributions::Standard)
            .take(64)
            .collect();
        STANDARD.encode(&random_bytes)
    }
}

/// Token Validator
///
/// Validates and decodes JWT access tokens, verifying signature, expiration, and claims.
/// Thread-safe and can be cloned for use across multiple tasks.
///
/// # Example
///
/// ```rust
/// use foundry_oauth_server::tokens::{TokenGenerator, TokenValidator};
/// use uuid::Uuid;
///
/// let secret = "your-secret-key".to_string();
/// let issuer = "https://auth.example.com".to_string();
///
/// let generator = TokenGenerator::new(secret.clone(), issuer.clone());
/// let validator = TokenValidator::new(secret, issuer);
///
/// let token = generator.generate_access_token(
///     Uuid::new_v4(),
///     None,
///     vec!["read".to_string()],
///     3600,
/// )?;
///
/// let claims = validator.validate_access_token(&token.token)?;
/// println!("Token is valid for scopes: {:?}", claims.scopes);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct TokenValidator {
    jwt_secret: String,
    issuer: String,
}

impl TokenValidator {
    /// Create a new token validator
    ///
    /// # Arguments
    ///
    /// * `jwt_secret` - Secret key used to verify JWT signatures (must match generator secret)
    /// * `issuer` - Expected issuer identifier for validation
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::TokenValidator;
    ///
    /// let validator = TokenValidator::new(
    ///     "your-secret-key".to_string(),
    ///     "https://auth.example.com".to_string()
    /// );
    /// ```
    pub fn new(jwt_secret: String, issuer: String) -> Self {
        Self { jwt_secret, issuer }
    }

    /// Validate and decode a JWT access token
    ///
    /// Verifies the token signature, expiration, issuer, and token type.
    /// Returns the decoded claims if validation succeeds.
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT access token string to validate
    ///
    /// # Returns
    ///
    /// Returns `TokenClaims` containing all claims if the token is valid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::{TokenGenerator, TokenValidator};
    /// use uuid::Uuid;
    ///
    /// let secret = "secret".to_string();
    /// let issuer = "issuer".to_string();
    ///
    /// let generator = TokenGenerator::new(secret.clone(), issuer.clone());
    /// let validator = TokenValidator::new(secret, issuer);
    ///
    /// let token = generator.generate_access_token(
    ///     Uuid::new_v4(),
    ///     Some(Uuid::new_v4()),
    ///     vec!["read".to_string()],
    ///     3600,
    /// )?;
    ///
    /// let claims = validator.validate_access_token(&token.token)?;
    ///
    /// println!("User ID: {:?}", claims.user_id);
    /// println!("Scopes: {:?}", claims.scopes);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token signature is invalid (`OAuth2Error::JwtError`)
    /// - Token has expired (`OAuth2Error::TokenExpired`)
    /// - Issuer doesn't match (`OAuth2Error::JwtError`)
    /// - Token type is not "access_token" (`OAuth2Error::InvalidRequest`)
    ///
    /// # Security
    ///
    /// This function uses constant-time comparison for cryptographic operations
    /// to prevent timing attacks.
    pub fn validate_access_token(&self, token: &str) -> OAuth2Result<TokenClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<TokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )?;

        // Additional validation
        if token_data.claims.token_type != "access_token" {
            return Err(OAuth2Error::InvalidRequest(
                "Invalid token type".to_string(),
            ));
        }

        // Check expiration
        let now = Utc::now().timestamp();
        if token_data.claims.exp < now {
            return Err(OAuth2Error::TokenExpired("Token expired".to_string()));
        }

        Ok(token_data.claims)
    }

    /// Introspect a token (RFC 7662)
    ///
    /// Returns metadata about a token, including whether it's active and its claims.
    /// This method never fails - inactive or invalid tokens return `active: false`.
    ///
    /// # Arguments
    ///
    /// * `token` - The JWT access token string to introspect
    ///
    /// # Returns
    ///
    /// Returns a `TokenIntrospection` response with token metadata.
    ///
    /// # Example
    ///
    /// ```rust
    /// use foundry_oauth_server::tokens::{TokenGenerator, TokenValidator};
    /// use uuid::Uuid;
    ///
    /// let secret = "secret".to_string();
    /// let issuer = "issuer".to_string();
    ///
    /// let generator = TokenGenerator::new(secret.clone(), issuer.clone());
    /// let validator = TokenValidator::new(secret, issuer);
    ///
    /// let token = generator.generate_access_token(
    ///     Uuid::new_v4(),
    ///     None,
    ///     vec!["read".to_string()],
    ///     3600,
    /// )?;
    ///
    /// let introspection = validator.introspect(&token.token);
    /// assert!(introspection.active);
    /// assert_eq!(introspection.scope, Some("read".to_string()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Note
    ///
    /// Invalid or expired tokens return `active: false` with all other fields as `None`.
    pub fn introspect(&self, token: &str) -> TokenIntrospection {
        match self.validate_access_token(token) {
            Ok(claims) => TokenIntrospection {
                active: true,
                scope: Some(claims.scopes.join(" ")),
                client_id: Some(claims.client_id),
                username: claims.user_id,
                token_type: Some("Bearer".to_string()),
                exp: Some(claims.exp),
                iat: Some(claims.iat),
                nbf: Some(claims.nbf),
                sub: Some(claims.sub),
                aud: claims.aud,
                iss: Some(claims.iss),
                jti: Some(claims.jti),
            },
            Err(_) => TokenIntrospection {
                active: false,
                scope: None,
                client_id: None,
                username: None,
                token_type: None,
                exp: None,
                iat: None,
                nbf: None,
                sub: None,
                aud: None,
                iss: None,
                jti: None,
            },
        }
    }
}

/// Token Introspection Response (RFC 7662)
///
/// Provides metadata about a token's validity and claims. Used by resource servers
/// to validate tokens without maintaining token state.
///
/// # Example
///
/// ```rust
/// use foundry_oauth_server::tokens::{TokenGenerator, TokenValidator};
/// use uuid::Uuid;
///
/// let generator = TokenGenerator::new("secret".to_string(), "issuer".to_string());
/// let validator = TokenValidator::new("secret".to_string(), "issuer".to_string());
///
/// let token = generator.generate_access_token(
///     Uuid::new_v4(),
///     None,
///     vec!["read".to_string()],
///     3600,
/// )?;
///
/// let introspection = validator.introspect(&token.token);
/// if introspection.active {
///     println!("Token is valid");
///     println!("Scopes: {:?}", introspection.scope);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIntrospection {
    /// Whether the token is currently active and valid
    pub active: bool,

    /// Scope
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Client ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// Username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Token type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,

    /// Expiration time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,

    /// Issued at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,

    /// Not before
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,

    /// Subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,

    /// Audience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<Vec<String>>,

    /// Issuer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// JWT ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_access_token() {
        let generator = TokenGenerator::new(
            "test_secret_key_1234567890".to_string(),
            "test-issuer".to_string(),
        );

        let client_id = Uuid::new_v4();
        let user_id = Some(Uuid::new_v4());
        let scopes = vec!["read".to_string(), "write".to_string()];

        let token = generator
            .generate_access_token(client_id, user_id, scopes.clone(), 3600)
            .unwrap();

        assert_eq!(token.client_id, client_id);
        assert_eq!(token.user_id, user_id);
        assert_eq!(token.scopes, scopes);
        assert!(!token.revoked);
    }

    #[test]
    fn test_validate_access_token() {
        let secret = "test_secret_key_1234567890".to_string();
        let issuer = "test-issuer".to_string();

        let generator = TokenGenerator::new(secret.clone(), issuer.clone());
        let validator = TokenValidator::new(secret, issuer);

        let client_id = Uuid::new_v4();
        let user_id = Some(Uuid::new_v4());
        let scopes = vec!["read".to_string()];

        let token = generator
            .generate_access_token(client_id, user_id, scopes.clone(), 3600)
            .unwrap();

        let claims = validator.validate_access_token(&token.token).unwrap();

        assert_eq!(claims.client_id, client_id.to_string());
        assert_eq!(claims.user_id, user_id.map(|id| id.to_string()));
        assert_eq!(claims.scopes, scopes);
        assert_eq!(claims.token_type, "access_token");
    }

    #[test]
    fn test_token_introspection() {
        let secret = "test_secret_key_1234567890".to_string();
        let issuer = "test-issuer".to_string();

        let generator = TokenGenerator::new(secret.clone(), issuer.clone());
        let validator = TokenValidator::new(secret, issuer);

        let client_id = Uuid::new_v4();
        let scopes = vec!["read".to_string()];

        let token = generator
            .generate_access_token(client_id, None, scopes, 3600)
            .unwrap();

        let introspection = validator.introspect(&token.token);

        assert!(introspection.active);
        assert_eq!(introspection.client_id, Some(client_id.to_string()));
        assert_eq!(introspection.token_type, Some("Bearer".to_string()));
    }

    #[test]
    fn test_expired_token() {
        let secret = "test_secret_key_1234567890".to_string();
        let issuer = "test-issuer".to_string();

        let generator = TokenGenerator::new(secret.clone(), issuer.clone());
        let validator = TokenValidator::new(secret, issuer);

        let client_id = Uuid::new_v4();
        let scopes = vec!["read".to_string()];

        // Generate token with negative lifetime (already expired)
        let token = generator
            .generate_access_token(client_id, None, scopes, -3600)
            .unwrap();

        let result = validator.validate_access_token(&token.token);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_refresh_token() {
        let generator = TokenGenerator::new(
            "test_secret_key_1234567890".to_string(),
            "test-issuer".to_string(),
        );

        let access_token_id = Uuid::new_v4();
        let refresh_token = generator
            .generate_refresh_token(access_token_id, 2592000)
            .unwrap();

        assert_eq!(refresh_token.access_token_id, access_token_id);
        assert!(!refresh_token.revoked);
        assert!(!refresh_token.token.is_empty());
    }
}
