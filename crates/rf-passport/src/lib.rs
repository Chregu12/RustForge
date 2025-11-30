//! # rf-passport
//!
//! Laravel Passport-style OAuth2 server for RustForge.
//!
//! ## Features
//!
//! - **Complete OAuth2 Server**: Full implementation of OAuth2 specification (RFC 6749)
//! - **Multiple Grant Types**:
//!   - Authorization Code with PKCE (RFC 7636)
//!   - Password Grant (Resource Owner Password Credentials)
//!   - Client Credentials
//!   - Implicit (deprecated, for compatibility)
//!   - Refresh Token
//! - **Personal Access Tokens**: Issue tokens without full OAuth flow
//! - **Scope Management**: Fine-grained permission control
//! - **Client Management**: Create and manage OAuth clients
//! - **Token Management**: Full lifecycle management with revocation
//! - **PKCE Support**: Enhanced security for public clients
//! - **Axum Integration**: Built-in middleware and extractors
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rf_passport::{PassportConfig, HasApiTokens};
//!
//! // 1. Configure Passport
//! let config = PassportConfig::new()
//!     .access_token_lifetime(3600)
//!     .enforce_pkce(true)
//!     .enable_password_grant(false);
//!
//! // 2. Create Personal Access Token
//! let token = user.create_token(
//!     "mobile-app",
//!     vec!["read:posts".to_string(), "write:posts".to_string()],
//!     &db,
//!     &config
//! ).await?;
//!
//! // 3. Protect Routes
//! use rf_passport::PassportAuth;
//!
//! async fn protected_route(
//!     PassportAuth(user_id, token): PassportAuth
//! ) -> impl IntoResponse {
//!     Json(json!({
//!         "user_id": user_id,
//!         "scopes": token.get_scopes()
//!     }))
//! }
//! ```
//!
//! ## OAuth2 Flows
//!
//! ### Authorization Code Flow (with PKCE)
//!
//! ```rust,ignore
//! use rf_passport::{AuthorizationCodeGrant, AuthorizationRequest};
//!
//! // Step 1: Authorization Request
//! let request = AuthorizationRequest {
//!     response_type: "code".to_string(),
//!     client_id: 1,
//!     redirect_uri: "http://localhost:3000/callback".to_string(),
//!     scope: Some("read:posts write:posts".to_string()),
//!     state: Some("random-state".to_string()),
//!     code_challenge: Some("challenge".to_string()),
//!     code_challenge_method: Some("S256".to_string()),
//! };
//!
//! let grant = AuthorizationCodeGrant::new(&db, &config);
//! let response = grant.authorize(user_id, request).await?;
//!
//! // Step 2: Token Exchange
//! let token_request = AuthorizationCodeTokenRequest {
//!     grant_type: "authorization_code".to_string(),
//!     code: response.code,
//!     redirect_uri: "http://localhost:3000/callback".to_string(),
//!     client_id: 1,
//!     client_secret: Some("secret".to_string()),
//!     code_verifier: Some("verifier".to_string()),
//! };
//!
//! let tokens = grant.exchange_token(token_request).await?;
//! ```
//!
//! ### Password Grant
//!
//! ```rust,ignore
//! use rf_passport::{PasswordGrant, PasswordVerifier};
//!
//! struct MyPasswordVerifier;
//!
//! #[async_trait]
//! impl PasswordVerifier for MyPasswordVerifier {
//!     async fn verify(&self, username: &str, password: &str) -> PassportResult<i64> {
//!         // Verify credentials and return user ID
//!         Ok(user_id)
//!     }
//! }
//!
//! let grant = PasswordGrant::new(&db, &config);
//! let tokens = grant.issue_token(request, &MyPasswordVerifier).await?;
//! ```
//!
//! ### Client Credentials
//!
//! ```rust,ignore
//! use rf_passport::ClientCredentialsGrant;
//!
//! let grant = ClientCredentialsGrant::new(&db, &config);
//! let tokens = grant.issue_token(request).await?;
//! ```
//!
//! ## Scope Management
//!
//! ```rust,ignore
//! use rf_passport::{Scope, ScopeRepository, register_scopes};
//!
//! // Register scopes
//! register_scopes! {
//!     "read:posts" => "Read blog posts",
//!     "write:posts" => "Create and edit blog posts",
//!     "delete:posts" => "Delete blog posts",
//! }
//!
//! // Check scope in route
//! async fn delete_post(
//!     PassportAuth(user_id, token): PassportAuth
//! ) -> impl IntoResponse {
//!     if !token.has_scope("delete:posts") {
//!         return Err(PassportError::InvalidScope("...".to_string()));
//!     }
//!     // Delete post
//! }
//! ```
//!
//! ## Database Schema
//!
//! Required tables:
//! - `oauth_clients`
//! - `oauth_access_tokens`
//! - `oauth_refresh_tokens`
//! - `oauth_auth_codes`
//!
//! See migration files for schema details.

pub mod auth_code;
pub mod client;
pub mod config;
pub mod errors;
pub mod grants;
pub mod handlers;
pub mod middleware;
pub mod personal_access_token;
pub mod scope;
pub mod token;

pub use auth_code::{
    generate_code_challenge, generate_code_verifier, verify_code_challenge, AuthCodeRepository,
    CodeChallengeMethod,
};
pub use client::{ClientRepository, Model as OAuthClient};
pub use config::PassportConfig;
pub use errors::{PassportError, PassportResult};
pub use grants::{
    AuthorizationCodeGrant, AuthorizationCodeTokenRequest, AuthorizationRequest,
    AuthorizationResponse, ClientCredentialsGrant, ClientCredentialsRequest, ImplicitGrant,
    ImplicitGrantRequest, ImplicitGrantResponse, PasswordGrant, PasswordGrantRequest,
    PasswordVerifier, RefreshTokenGrant, RefreshTokenRequest, TokenResponse,
};
pub use handlers::{
    create_client, delete_client, list_clients, list_tokens, revoke_token, token_endpoint,
    ClientInfo, CreateClientRequest, CreateClientResponse, PassportState, TokenInfo,
};
pub use middleware::{check_any_scope, check_scopes, DatabaseExtension, PassportAuth};
pub use personal_access_token::HasApiTokens;
pub use scope::{Scope, ScopeChecker, ScopeRepository};
pub use token::{OAuthAccessToken, OAuthRefreshToken, TokenRepository};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        check_any_scope, check_scopes, generate_code_challenge, generate_code_verifier,
        verify_code_challenge, AuthCodeRepository, AuthorizationCodeGrant,
        AuthorizationCodeTokenRequest, AuthorizationRequest, AuthorizationResponse,
        ClientCredentialsGrant, ClientCredentialsRequest, ClientRepository, CodeChallengeMethod,
        DatabaseExtension, HasApiTokens, ImplicitGrant, ImplicitGrantRequest,
        ImplicitGrantResponse, OAuthAccessToken, OAuthClient, OAuthRefreshToken, PassportAuth,
        PassportConfig, PassportError, PassportResult, PassportState, PasswordGrant,
        PasswordGrantRequest, PasswordVerifier, RefreshTokenGrant, RefreshTokenRequest, Scope,
        ScopeChecker, ScopeRepository, TokenRepository, TokenResponse,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passport_compiles() {
        // Basic compilation test
        assert!(true);
    }

    #[test]
    fn test_config_builder() {
        let config = PassportConfig::new()
            .access_token_lifetime(7200)
            .enforce_pkce(true)
            .enable_password_grant(false);

        assert_eq!(config.access_token_lifetime, 7200);
        assert_eq!(config.enforce_pkce, true);
        assert_eq!(config.enable_password_grant, false);
    }
}
