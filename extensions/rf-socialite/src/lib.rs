//! OAuth2 and social login for RustForge
//!
//! This crate provides OAuth2 authentication and social login integration,
//! similar to Laravel Socialite.
//!
//! # Supported Providers
//!
//! - GitHub
//! - Google
//! - Facebook
//! - Twitter/X
//! - Generic OAuth2
//!
//! # Quick Start
//!
//! ```rust
//! use rf_socialite::{Socialite, Provider};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut github = Socialite::driver(Provider::GitHub)
//!     .client_id("your-client-id")
//!     .client_secret("your-client-secret")
//!     .redirect_url("http://localhost:8000/auth/callback")
//!     .build()?;
//!
//! // Redirect user to provider
//! let auth_url = github.redirect()?;
//!
//! // Handle callback
//! let user = github.user_from_code("auth-code-from-callback").await?;
//! println!("Logged in as: {}", user.name);
//! # Ok(())
//! # }
//! ```

pub mod account_linking;
pub mod config;
mod driver;
pub mod manager;
pub mod pkce;
pub mod providers;
pub mod routes;
pub mod state;
mod user;

pub use account_linking::{AccountLinker, LinkingStrategy, SocialAccount};
pub use config::{ProviderConfig, SocialiteConfig};
pub use driver::{Driver, DriverBuilder, Socialite, SocialiteError, TokenResponse};
pub use manager::SocialiteManager;
pub use pkce::Pkce;
pub use providers::Provider;
pub use state::StateManager;
pub use user::{User, UserData};

pub type Result<T> = std::result::Result<T, SocialiteError>;
