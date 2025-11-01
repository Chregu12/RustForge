//! JWT (JSON Web Token) implementation for authentication
//!
//! This module provides secure token generation, validation, and refresh functionality
//! using the jsonwebtoken crate. It supports both access tokens (short-lived) and
//! refresh tokens (long-lived) for a complete authentication flow.

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::guard::AuthError;

/// JWT Claims structure containing user identity and token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// User name
    pub name: String,
    /// Issued at (timestamp)
    pub iat: i64,
    /// Expiration time (timestamp)
    pub exp: i64,
    /// Token type (access or refresh)
    pub token_type: TokenType,
    /// JWT ID (unique identifier for this token)
    pub jti: String,
}

impl Claims {
    /// Create new claims for a user
    pub fn new(
        user_id: i64,
        email: String,
        name: String,
        token_type: TokenType,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        let exp = (now + expiration).timestamp();

        Self {
            sub: user_id.to_string(),
            email,
            name,
            iat: now.timestamp(),
            exp,
            token_type,
            jti: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        self.exp < now
    }

    /// Get expiration as DateTime
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(Utc::now)
    }

    /// Get user ID as i64
    pub fn user_id(&self) -> Result<i64, AuthError> {
        self.sub
            .parse::<i64>()
            .map_err(|_| AuthError::Internal("Invalid user ID in token".to_string()))
    }
}

/// Token type discriminator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::Access => write!(f, "access"),
            TokenType::Refresh => write!(f, "refresh"),
        }
    }
}

/// JWT token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// JWT configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for signing tokens
    pub secret: String,
    /// Access token expiration duration (default: 15 minutes)
    pub access_token_ttl: Duration,
    /// Refresh token expiration duration (default: 7 days)
    pub refresh_token_ttl: Duration,
    /// Issuer name
    pub issuer: Option<String>,
    /// Audience
    pub audience: Option<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            access_token_ttl: Duration::minutes(15),
            refresh_token_ttl: Duration::days(7),
            issuer: None,
            audience: None,
        }
    }
}

impl JwtConfig {
    /// Create a new JWT config with custom secret
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ..Default::default()
        }
    }

    /// Set access token TTL
    pub fn with_access_ttl(mut self, ttl: Duration) -> Self {
        self.access_token_ttl = ttl;
        self
    }

    /// Set refresh token TTL
    pub fn with_refresh_ttl(mut self, ttl: Duration) -> Self {
        self.refresh_token_ttl = ttl;
        self
    }

    /// Set issuer
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set audience
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }
}

/// JWT service for token management
#[derive(Clone)]
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new JWT service with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Create a new JWT service with default configuration
    pub fn default() -> Self {
        Self::new(JwtConfig::default())
    }

    /// Generate a token pair (access + refresh) for a user
    pub fn generate_token_pair(
        &self,
        user_id: i64,
        email: String,
        name: String,
    ) -> Result<TokenPair, AuthError> {
        let access_token = self.generate_access_token(user_id, email.clone(), name.clone())?;
        let refresh_token = self.generate_refresh_token(user_id, email, name)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_ttl.num_seconds(),
        })
    }

    /// Generate an access token
    pub fn generate_access_token(
        &self,
        user_id: i64,
        email: String,
        name: String,
    ) -> Result<String, AuthError> {
        let claims = Claims::new(
            user_id,
            email,
            name,
            TokenType::Access,
            self.config.access_token_ttl,
        );

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Failed to encode token: {}", e)))
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(
        &self,
        user_id: i64,
        email: String,
        name: String,
    ) -> Result<String, AuthError> {
        let claims = Claims::new(
            user_id,
            email,
            name,
            TokenType::Refresh,
            self.config.refresh_token_ttl,
        );

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AuthError::Internal(format!("Failed to encode token: {}", e)))
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let mut validation = Validation::default();

        // Set issuer and audience if configured
        if let Some(ref issuer) = self.config.issuer {
            validation.set_issuer(&[issuer]);
        }
        if let Some(ref audience) = self.config.audience {
            validation.set_audience(&[audience]);
        }

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::SessionExpired,
                jsonwebtoken::errors::ErrorKind::InvalidToken => AuthError::InvalidCredentials,
                _ => AuthError::Internal(format!("Token validation failed: {}", e)),
            })?;

        Ok(token_data.claims)
    }

    /// Validate an access token
    pub fn validate_access_token(&self, token: &str) -> Result<Claims, AuthError> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Access {
            return Err(AuthError::Internal("Invalid token type".to_string()));
        }

        Ok(claims)
    }

    /// Validate a refresh token
    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims, AuthError> {
        let claims = self.validate_token(token)?;

        if claims.token_type != TokenType::Refresh {
            return Err(AuthError::Internal("Invalid token type".to_string()));
        }

        Ok(claims)
    }

    /// Refresh an access token using a refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<String, AuthError> {
        let claims = self.validate_refresh_token(refresh_token)?;

        self.generate_access_token(
            claims.user_id()?,
            claims.email.clone(),
            claims.name.clone(),
        )
    }

    /// Get the access token TTL in seconds
    pub fn access_token_ttl_seconds(&self) -> i64 {
        self.config.access_token_ttl.num_seconds()
    }

    /// Get the refresh token TTL in seconds
    pub fn refresh_token_ttl_seconds(&self) -> i64 {
        self.config.refresh_token_ttl.num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_access_token() {
        let service = JwtService::new(JwtConfig::new("test-secret"));

        let token = service
            .generate_access_token(1, "test@example.com".to_string(), "Test User".to_string())
            .expect("Failed to generate token");

        let claims = service
            .validate_access_token(&token)
            .expect("Failed to validate token");

        assert_eq!(claims.sub, "1");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.name, "Test User");
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_generate_token_pair() {
        let service = JwtService::new(JwtConfig::new("test-secret"));

        let pair = service
            .generate_token_pair(1, "test@example.com".to_string(), "Test User".to_string())
            .expect("Failed to generate token pair");

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");

        // Validate both tokens
        let access_claims = service
            .validate_access_token(&pair.access_token)
            .expect("Failed to validate access token");
        assert_eq!(access_claims.token_type, TokenType::Access);

        let refresh_claims = service
            .validate_refresh_token(&pair.refresh_token)
            .expect("Failed to validate refresh token");
        assert_eq!(refresh_claims.token_type, TokenType::Refresh);
    }

    #[test]
    fn test_refresh_access_token() {
        let service = JwtService::new(JwtConfig::new("test-secret"));

        let refresh_token = service
            .generate_refresh_token(1, "test@example.com".to_string(), "Test User".to_string())
            .expect("Failed to generate refresh token");

        let new_access_token = service
            .refresh_access_token(&refresh_token)
            .expect("Failed to refresh access token");

        let claims = service
            .validate_access_token(&new_access_token)
            .expect("Failed to validate new access token");

        assert_eq!(claims.sub, "1");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new(JwtConfig::new("test-secret"));

        let result = service.validate_token("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_token_type() {
        let service = JwtService::new(JwtConfig::new("test-secret"));

        let access_token = service
            .generate_access_token(1, "test@example.com".to_string(), "Test User".to_string())
            .expect("Failed to generate token");

        // Try to validate access token as refresh token
        let result = service.validate_refresh_token(&access_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_expiration() {
        let claims = Claims::new(
            1,
            "test@example.com".to_string(),
            "Test User".to_string(),
            TokenType::Access,
            Duration::seconds(-1), // Already expired
        );

        assert!(claims.is_expired());
    }
}
