//! # rf-sanctum
//!
//! Laravel Sanctum-style API token authentication with per-token abilities.
//!
//! ## Features
//!
//! - Personal Access Tokens (PAT)
//! - Token Abilities/Scopes
//! - Token Expiration
//! - Token Last Used Tracking
//! - SPA Cookie Authentication
//! - Database persistence with SeaORM
//!
//! ## Usage
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
//! // Use in routes
//! async fn protected(SanctumAuth(user, token): SanctumAuth<User>) -> Json<User> {
//!     Json(user)
//! }
//! ```

pub mod abilities;
pub mod auth;
pub mod errors;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod spa;
pub mod token;
pub mod tokenable;

pub use abilities::{Ability, AbilityChecker};
pub use auth::{LoadFromToken, SanctumAuth};
pub use errors::SanctumError;
pub use models::Model as PersonalAccessTokenModel;
pub use repository::TokenRepository;
pub use token::{NewToken, PersonalAccessToken};
pub use tokenable::Tokenable;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Ability, AbilityChecker, LoadFromToken, NewToken, PersonalAccessToken,
        PersonalAccessTokenModel, SanctumAuth, SanctumError, TokenRepository, Tokenable,
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
