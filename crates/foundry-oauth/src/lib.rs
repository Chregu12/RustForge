//! # Foundry OAuth / SSO
//!
//! OAuth 2.0 and SSO integration for Google, GitHub, Facebook, OpenID Connect.
//!
//! # Features
//! - Multiple provider support (Google, GitHub, Facebook)
//! - State parameter validation for CSRF protection
//! - Token refresh handling
//! - Comprehensive error handling
//! - Async/await support
//!
//! # Example
//! ```rust,ignore
//! use foundry_oauth::{OAuthClient, GoogleProvider};
//!
//! let mut client = OAuthClient::new();
//! let provider = GoogleProvider::new(
//!     "client_id".to_string(),
//!     "client_secret".to_string(),
//!     "http://localhost:8000/auth/google/callback".to_string()
//! );
//!
//! client.register_provider(Box::new(provider));
//!
//! // Redirect user to provider
//! let auth_url = client.get_authorize_url("google", "state123").unwrap();
//!
//! // After callback, exchange code for token
//! let user = client.authenticate("google", "code", "state123").await?;
//! ```

pub mod providers;
pub mod traits;
pub mod client;
pub mod state;

pub use providers::{GoogleProvider, GithubProvider, FacebookProvider};
pub use traits::{OAuthProvider, OAuthUser, OAuthTokens};
pub use client::OAuthClient;
pub use state::StateManager;

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Invalid state parameter")]
    InvalidState,

    #[error("State expired")]
    StateExpired,

    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("JSON parsing failed: {0}")]
    JsonError(String),

    #[error("Token refresh failed: {0}")]
    RefreshError(String),
}

pub type Result<T> = std::result::Result<T, OAuthError>;
