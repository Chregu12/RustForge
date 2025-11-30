//! # rf-sanctum-facade
//!
//! Laravel-style Sanctum facade for the RustForge framework.
//!
//! This crate provides a static, fluent API for Sanctum token authentication similar to Laravel's
//! Sanctum facade, making it easy to work with API tokens from anywhere in your application.
//!
//! ## Features
//!
//! - **Static Sanctum API**: Use `Sanctum::tokenCan()`, `Sanctum::currentAccessToken()`, etc.
//! - **Global Token Management**: Thread-safe global token state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//! - **Token Abilities**: Fine-grained permission control
//! - **Token Lifecycle**: Create, revoke, and manage tokens
//! - **Token Pruning**: Clean up expired and unused tokens
//! - **Token Statistics**: Analytics on token usage
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//! use serde::{Serialize, Deserialize};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Check if current token has ability
//! if Sanctum::tokenCan("read:posts").await {
//!     println!("User can read posts");
//! }
//!
//! // Get current access token
//! if let Some(token) = Sanctum::currentAccessToken().await {
//!     println!("Token: {}", token.name);
//! }
//!
//! // Create token for user
//! // let token = Sanctum::createToken(&user, "mobile-app", vec!["*"], None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Token Management
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//! use rf_sanctum::Tokenable;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create token with abilities
//! // let token = Sanctum::createToken(
//! //     &user,
//! //     "api-token",
//! //     vec!["read:posts", "write:posts"],
//! //     None
//! // ).await?;
//!
//! // Create token with device info
//! // let token = Sanctum::createTokenWithDevice(
//! //     &user,
//! //     "mobile-app",
//! //     vec!["*"],
//! //     None,
//! //     Some("Mozilla/5.0...".to_string()),
//! //     Some("192.168.1.1".to_string())
//! // ).await?;
//!
//! // Revoke current token
//! // Sanctum::revokeCurrentToken().await?;
//!
//! // Revoke all user tokens
//! // Sanctum::revokeAllTokens(&user).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Token Abilities
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//!
//! # async fn example() {
//! // Check single ability
//! if Sanctum::tokenCan("read:posts").await {
//!     // User can read posts
//! }
//!
//! // Check any of multiple abilities
//! if Sanctum::tokenCanAny(&["read:posts", "write:posts"]).await {
//!     // User can read OR write posts
//! }
//!
//! // Check all abilities
//! if Sanctum::tokenCanAll(&["read:posts", "write:posts"]).await {
//!     // User can read AND write posts
//! }
//! # }
//! ```
//!
//! ## Token Pruning
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Prune expired tokens
//! let deleted = Sanctum::pruneExpiredTokens().await?;
//! println!("Deleted {} expired tokens", deleted);
//!
//! // Prune tokens older than 90 days
//! let deleted = Sanctum::pruneTokensOlderThan(90).await?;
//!
//! // Prune unused tokens (not used in 30 days)
//! let deleted = Sanctum::pruneUnusedTokens(30).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Token Statistics
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Get token stats for user
//! // let stats = Sanctum::tokenStats(&user).await?;
//! // println!("Total: {}, Active: {}, Expired: {}",
//! //     stats.total, stats.active, stats.expired);
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! ```rust,no_run
//! use rf_sanctum_facade::Sanctum;
//! use sea_orm::DatabaseConnection;
//! use std::sync::Arc;
//!
//! # async fn example(db: DatabaseConnection) {
//! // Set database connection
//! Sanctum::setDatabase(Arc::new(db)).await;
//! # }
//! ```

pub mod facade;
pub mod manager;

pub use facade::Sanctum;
pub use manager::{SanctumManager, GLOBAL_SANCTUM};

// Re-export commonly used types from rf-sanctum
pub use rf_sanctum::{
    Ability, AbilityChecker, AuthMethod, HashAlgorithm, LoadFromToken, NewToken,
    PersonalAccessToken, PersonalAccessTokenModel, SanctumAuth, SanctumConfig, SanctumError,
    SanctumGuard, Tokenable, TokenRepository, TokenStats, TransientTokenBuilder,
    TransientTokenStore,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanctum_facade_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
