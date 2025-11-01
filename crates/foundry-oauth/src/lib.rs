//! # Foundry OAuth / SSO
//!
//! OAuth 2.0 and SSO integration for Google, GitHub, Facebook, OpenID Connect.

pub mod providers;
pub mod traits;
pub mod client;

pub use providers::{GoogleProvider, GithubProvider, FacebookProvider};
pub use traits::{OAuthProvider, OAuthUser};
pub use client::OAuthClient;

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Provider error: {0}")]
    ProviderError(String),
}

pub type Result<T> = std::result::Result<T, OAuthError>;
