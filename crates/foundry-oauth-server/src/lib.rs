//! # Foundry OAuth2 Server
//!
//! Laravel Passport equivalent for Rust - Full OAuth2 Authorization Server
//!
//! ## Features
//! - Authorization Code Flow
//! - Client Credentials Flow
//! - Password Grant Flow
//! - Refresh Token Flow
//! - Personal Access Tokens
//! - Scope Management
//! - Token Introspection
//! - Token Revocation

pub mod models;
pub mod grants;
pub mod tokens;
pub mod clients;
pub mod scopes;
pub mod server;
pub mod routes;
pub mod errors;
pub mod repositories;

pub use server::OAuth2Server;
pub use models::{Client, AccessToken, RefreshToken, AuthorizationCode, PersonalAccessToken};
pub use grants::{GrantType, AuthorizationCodeGrant, ClientCredentialsGrant, PasswordGrant, RefreshTokenGrant};
pub use tokens::{TokenGenerator, TokenValidator, TokenClaims};
pub use clients::ClientRepository;
pub use scopes::{Scope, ScopeManager};
pub use errors::{OAuth2Error, OAuth2Result};

/// OAuth2 Server Configuration
#[derive(Debug, Clone)]
pub struct OAuth2Config {
    /// Access token lifetime in seconds (default: 3600 = 1 hour)
    pub access_token_lifetime: i64,

    /// Refresh token lifetime in seconds (default: 2592000 = 30 days)
    pub refresh_token_lifetime: i64,

    /// Authorization code lifetime in seconds (default: 600 = 10 minutes)
    pub auth_code_lifetime: i64,

    /// Personal access token lifetime in seconds (default: 31536000 = 1 year)
    pub personal_access_token_lifetime: i64,

    /// JWT signing key
    pub jwt_secret: String,

    /// Issuer identifier
    pub issuer: String,

    /// Enable PKCE (Proof Key for Code Exchange)
    pub enable_pkce: bool,
}

impl Default for OAuth2Config {
    fn default() -> Self {
        // Generate 256-bit cryptographically secure JWT secret
        let mut secret_bytes = vec![0u8; 32];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut secret_bytes);

        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let jwt_secret = STANDARD.encode(&secret_bytes);

        Self {
            access_token_lifetime: 3600,
            refresh_token_lifetime: 2592000,
            auth_code_lifetime: 600,
            personal_access_token_lifetime: 31536000,
            jwt_secret,
            issuer: "foundry-oauth-server".to_string(),
            enable_pkce: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OAuth2Config::default();
        assert_eq!(config.access_token_lifetime, 3600);
        assert_eq!(config.refresh_token_lifetime, 2592000);
        assert!(config.enable_pkce);
    }
}
