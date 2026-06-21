//! # rf-passport-facade
//!
//! Laravel-style Passport facade for the RustForge framework.
//!
//! This crate provides a static, fluent API for OAuth2 server similar to Laravel's
//! Passport facade, making it easy to work with OAuth2 from anywhere in your application.
//!
//! ## Features
//!
//! - **Static Passport API**: Use `Passport::tokensCan()`, `Passport::createToken()`, etc.
//! - **Global OAuth2 Configuration**: Thread-safe global OAuth2 state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//! - **Scope Management**: Fine-grained permission control
//! - **Personal Access Tokens**: Issue tokens without full OAuth flow
//! - **Client Management**: Create and manage OAuth clients
//! - **Token Lifecycle**: Create, revoke, and manage tokens
//! - **Grant Control**: Enable/disable OAuth2 grant types
//! - **PKCE Support**: Enhanced security configuration
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//! use chrono::Duration;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure token lifetimes
//! Passport::tokensExpireIn(Duration::seconds(3600 * 24 * 15));
//! Passport::refreshTokensExpireIn(Duration::seconds(3600 * 24 * 30));
//!
//! // Define scopes
//! Passport::tokensCan(&[
//!     ("read:posts", "Read blog posts"),
//!     ("write:posts", "Create and edit posts"),
//! ]);
//!
//! // Set default scopes
//! Passport::setDefaultScope(&["read:posts"]);
//! # Ok(())
//! # }
//! ```
//!
//! ## Scope Management
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//!
//! # fn example() {
//! // Define available scopes
//! Passport::tokensCan(&[
//!     ("read:posts", "Read blog posts"),
//!     ("write:posts", "Create and edit posts"),
//!     ("delete:posts", "Delete posts"),
//! ]);
//!
//! // Check if scope exists
//! if Passport::hasScope("read:posts") {
//!     println!("Scope is registered");
//! }
//!
//! // Get all scopes
//! let scopes = Passport::scopes();
//! for scope in scopes {
//!     println!("{}: {}", scope.id, scope.description);
//! }
//! # }
//! ```
//!
//! ## Personal Access Tokens
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create personal access token
//! // let token = Passport::createToken(
//! //     &user,
//! //     "api-token",
//! //     vec!["read:posts".to_string(), "write:posts".to_string()]
//! // )?;
//!
//! // Check current token scope
//! if Passport::tokenCan("write:posts") {
//!     // User can write posts
//! }
//!
//! // Revoke token
//! // Passport::revokeToken(token_id)?;
//!
//! // Revoke all user tokens
//! // let count = Passport::revokeAllTokens(&user)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Client Management
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create OAuth client (returns the client and its plain-text secret)
//! let (client, secret) = Passport::createClient(
//!     "My Application",
//!     "https://myapp.com/callback"
//! )?;
//!
//! println!("Client ID: {}", client.id);
//! println!("Client Secret: {:?}", secret);
//!
//! // List all clients for a user
//! let clients = Passport::clients(client.user_id.unwrap_or(0))?;
//!
//! // Delete client
//! Passport::deleteClient(client.id)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Grant Control
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//!
//! # fn example() {
//! // Enable/disable grants
//! Passport::enablePasswordGrant();
//! Passport::disableImplicitGrant();
//! Passport::enableClientCredentialsGrant();
//!
//! // PKCE configuration
//! Passport::requirePkce(true);
//! Passport::allowPlainPkce(false);
//! # }
//! ```
//!
//! ## Token Lifetimes
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//! use chrono::Duration;
//!
//! # fn example() {
//! // Configure various token lifetimes
//! Passport::tokensExpireIn(Duration::seconds(3600)); // 1 hour
//! Passport::refreshTokensExpireIn(Duration::seconds(3600 * 24 * 30)); // 30 days
//! Passport::personalAccessTokensExpireIn(Duration::seconds(3600 * 24 * 365)); // 1 year
//! Passport::authCodesExpireIn(Duration::seconds(600)); // 10 minutes
//! # }
//! ```
//!
//! ## Configuration
//!
//! ```rust,no_run
//! use rf_passport_facade::Passport;
//! use sea_orm::DatabaseConnection;
//! use std::sync::Arc;
//!
//! # fn example(db: DatabaseConnection) {
//! // Set database connection
//! Passport::setDatabase(Arc::new(db));
//!
//! // Get current config
//! let config = Passport::config();
//! println!("Access token lifetime: {}", config.access_token_lifetime);
//! # }
//! ```

// Mirrors Laravel Passport's camelCase facade API (`Passport::tokensCan()`,
// `Passport::tokensExpireIn()`, ...); non-snake-case names are intentional.
#![allow(non_snake_case)]

pub mod config;
pub mod facade;
pub mod manager;

pub use config::{GrantControl, PkceControl, TokenLifetimes};
pub use facade::Passport;
pub use manager::{PassportManager, GLOBAL_PASSPORT};

// Re-export commonly used types from rf-passport
pub use rf_passport::{
    check_any_scope, check_scopes, generate_code_challenge, generate_code_verifier,
    verify_code_challenge, AuthCodeRepository, AuthorizationCodeGrant,
    AuthorizationCodeTokenRequest, AuthorizationRequest, AuthorizationResponse,
    ClientCredentialsGrant, ClientCredentialsRequest, ClientRepository, CodeChallengeMethod,
    DatabaseExtension, HasApiTokens, ImplicitGrant, ImplicitGrantRequest, ImplicitGrantResponse,
    OAuthAccessToken, OAuthClient, OAuthRefreshToken, PassportAuth, PassportConfig, PassportError,
    PassportResult, PassportState, PasswordGrant, PasswordGrantRequest, PasswordVerifier,
    RefreshTokenGrant, RefreshTokenRequest, Scope, ScopeChecker, ScopeRepository, TokenRepository,
    TokenResponse,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        GrantControl, PkceControl, Passport, PassportManager, TokenLifetimes, GLOBAL_PASSPORT,
    };
    pub use rf_passport::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passport_facade_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
