//! # rf-oauth2-server: OAuth2 Server Implementation for RustForge
//!
//! Provides a complete OAuth2 authorization server with multiple grant types.
//!
//! ## Features
//!
//! - **Authorization Code Flow**: Standard OAuth2 flow for web applications
//! - **Client Credentials Flow**: Machine-to-machine authentication
//! - **Token Management**: JWT-based access and refresh tokens
//! - **PKCE Support**: Enhanced security for public clients
//! - **Scope Management**: Fine-grained access control
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_oauth2_server::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create OAuth2 server
//! let oauth = OAuth2Server::new(OAuth2Config {
//!     issuer: "https://auth.example.com".to_string(),
//!     access_token_ttl: 3600,
//!     refresh_token_ttl: 86400,
//! });
//!
//! // Register a client
//! oauth.register_client(Client {
//!     id: "client-123".to_string(),
//!     secret: Some("secret".to_string()),
//!     redirect_uris: vec!["https://app.example.com/callback".to_string()],
//!     grants: vec![GrantType::AuthorizationCode, GrantType::ClientCredentials],
//!     scopes: vec!["read".to_string(), "write".to_string()],
//! }).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod server;
mod token;
mod types;

pub use client::Client;
pub use error::{OAuth2Error, OAuth2Result};
pub use server::OAuth2Server;
pub use token::{AccessToken, RefreshToken, TokenResponse};
pub use types::{GrantType, OAuth2Config, Scope};
