//! # rf-sanctum
//!
//! Laravel Sanctum-style API token authentication with per-token abilities.
//!
//! ## Features
//!
//! - **Personal Access Tokens (PAT)** with SHA-256 hashing
//! - **Token Abilities/Scopes** with granular permission control
//! - **Token Expiration** with automatic cleanup
//! - **Device Tracking** - User agent and IP address tracking
//! - **Token Pruning** - Clean up old and expired tokens
//! - **SPA Cookie Authentication** - Support for session-based auth
//! - **Transient Tokens** - In-memory tokens for testing
//! - **SanctumGuard** - Unified authentication interface
//! - **Token Statistics** - Analytics on token usage
//! - **Database persistence** with SeaORM
//!
//! ## Usage
//!
//! ### Basic Token Authentication
//!
//! ```rust,ignore
//! use rf_sanctum::{Tokenable, SanctumAuth, LoadFromToken};
//!
//! // Implement Tokenable for your user model
//! #[async_trait]
//! impl Tokenable for User {
//!     fn tokenable_type() -> &'static str {
//!         "User"
//!     }
//!
//!     fn tokenable_id(&self) -> i64 {
//!         self.id
//!     }
//! }
//!
//! // Implement LoadFromToken
//! #[async_trait]
//! impl LoadFromToken for User {
//!     async fn load_from_token(
//!         tokenable_id: i64,
//!         db: &DatabaseConnection,
//!     ) -> Result<Self, SanctumError> {
//!         // Load user from database
//!     }
//! }
//!
//! // Create a token for a user
//! let new_token = user.create_token("mobile-app", vec!["read:posts", "write:posts"], None, &db).await?;
//! println!("Token: {}", new_token.access_token); // Show once to user
//!
//! // Use in routes with SanctumAuth
//! async fn protected(SanctumAuth(user, token): SanctumAuth<User>) -> Json<User> {
//!     Json(user)
//! }
//!
//! // Or use SanctumGuard for more flexibility
//! async fn protected(guard: SanctumGuard<User>) -> Json<User> {
//!     if guard.can("admin") {
//!         // Admin-only logic
//!     }
//!     Json(guard.user)
//! }
//! ```
//!
//! ### Token Pruning
//!
//! ```rust,ignore
//! use rf_sanctum::TokenRepository;
//!
//! let repo = TokenRepository::new(&db);
//!
//! // Prune expired tokens
//! let deleted = repo.prune_expired_tokens().await?;
//!
//! // Prune tokens older than 90 days
//! let deleted = repo.prune_tokens_older_than(90).await?;
//!
//! // Prune unused tokens (not used in 30 days)
//! let deleted = repo.prune_unused_tokens(30).await?;
//! ```
//!
//! ### Device Tracking
//!
//! ```rust,ignore
//! // Create token with device info
//! let new_token = user.create_token_with_device(
//!     "mobile-app",
//!     vec!["*"],
//!     None,
//!     Some("Mozilla/5.0...".to_string()),
//!     Some("192.168.1.1".to_string()),
//!     &db
//! ).await?;
//!
//! // Check device info
//! if let Some(device) = token.device_name() {
//!     println!("Device: {}", device); // "Mobile Device", "Desktop", etc.
//! }
//! ```
//!
//! ### Transient Tokens
//!
//! ```rust,ignore
//! use rf_sanctum::{TransientTokenStore, TransientTokenBuilder};
//!
//! let store = TransientTokenStore::new();
//!
//! // Create transient token
//! let (plain, token) = TransientTokenBuilder::new("User", 1, "test-token")
//!     .with_abilities(vec!["read:posts".to_string()])
//!     .build();
//!
//! store.store(token)?;
//! ```

pub mod abilities;
pub mod auth;
pub mod config;
pub mod errors;
pub mod facade;
pub mod guard;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod spa;
pub mod token;
pub mod tokenable;
pub mod transient;

pub use abilities::{Ability, AbilityChecker};
pub use auth::{LoadFromToken, SanctumAuth};
pub use config::{HashAlgorithm, SanctumConfig};
pub use errors::SanctumError;
pub use guard::{AuthMethod, SanctumGuard};
pub use models::Model as PersonalAccessTokenModel;
pub use repository::{TokenRepository, TokenStats};
pub use token::{NewToken, PersonalAccessToken};
pub use tokenable::Tokenable;
pub use transient::{TransientTokenBuilder, TransientTokenStore};

// Facade re-exports (Laravel-style static API)
pub use facade::{
    manager::{SanctumManager, GLOBAL_SANCTUM},
    sanctum::Sanctum,
};

// Dependency re-exports so callers do not need direct sea-orm / axum dev deps.
// Callers implementing LoadFromToken or custom extractors can use:
//   rf_sanctum::DatabaseConnection  (sea_orm::DatabaseConnection)
//   rf_sanctum::FromRequestParts    (axum::extract::FromRequestParts)
pub use sea_orm::DatabaseConnection;
pub use axum::extract::FromRequestParts;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Ability, AbilityChecker, AuthMethod, DatabaseConnection, FromRequestParts, HashAlgorithm,
        LoadFromToken, NewToken, PersonalAccessToken, PersonalAccessTokenModel, SanctumAuth,
        SanctumConfig, SanctumError, SanctumGuard, TokenRepository, TokenStats, Tokenable,
        TransientTokenBuilder, TransientTokenStore,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctum_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
