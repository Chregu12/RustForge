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

mod driver;
mod user;
pub mod providers;
pub mod pkce;
pub mod state;
pub mod config;
pub mod account_linking;
pub mod manager;
pub mod routes;

pub use driver::{Driver, DriverBuilder, Socialite, SocialiteError, TokenResponse};
pub use user::{User, UserData};
pub use providers::Provider;
pub use manager::SocialiteManager;
pub use config::{ProviderConfig, SocialiteConfig};
pub use account_linking::{SocialAccount, LinkingStrategy, AccountLinker};
pub use state::StateManager;
pub use pkce::Pkce;

pub type Result<T> = std::result::Result<T, SocialiteError>;
