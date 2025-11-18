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
//!
//! ## Usage
//!
//! ```rust,ignore
//! use rf_sanctum::{Tokenable, SanctumAuth};
//!
//! // Create a token for a user
//! let token = user.create_token("mobile-app", vec!["read:posts", "write:posts"]).await?;
//!
//! // Use in routes
//! async fn protected(user: SanctumAuth<User>) -> Json<User> {
//!     Json(user.0)
//! }
//! ```

pub mod token;
pub mod tokenable;
pub mod auth;
pub mod abilities;
pub mod errors;

pub use token::{PersonalAccessToken, NewToken};
pub use tokenable::Tokenable;
pub use auth::SanctumAuth;
pub use abilities::{Ability, AbilityChecker};
pub use errors::SanctumError;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Ability, AbilityChecker, NewToken, PersonalAccessToken, SanctumAuth, SanctumError,
        Tokenable,
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
