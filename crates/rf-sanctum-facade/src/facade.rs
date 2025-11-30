//! Sanctum facade providing Laravel-style static API for token authentication

use crate::manager::GLOBAL_SANCTUM;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rf_sanctum::{
    LoadFromToken, NewToken, PersonalAccessToken, SanctumError, Tokenable, TokenRepository,
    TokenStats,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// The Sanctum facade providing a static-like API for token authentication.
///
/// This is the main entry point for Sanctum token management in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_sanctum_facade::Sanctum;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Check if user has a specific token ability
/// if Sanctum::tokenCan("read:posts").await {
///     println!("User can read posts");
/// }
///
/// // Get current access token
/// let token = Sanctum::currentAccessToken().await;
/// # Ok(())
/// # }
/// ```
pub struct Sanctum;

impl Sanctum {
    /// Configure Sanctum with a database connection
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    /// use sea_orm::DatabaseConnection;
    /// use std::sync::Arc;
    ///
    /// # async fn example(db: DatabaseConnection) {
    /// Sanctum::setDatabase(Arc::new(db)).await;
    /// # }
    /// ```
    pub async fn setDatabase(db: Arc<DatabaseConnection>) {
        let mut manager = GLOBAL_SANCTUM.write().await;
        manager.set_database(db);
    }

    /// Check if the current request is authenticated via Sanctum
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// if Sanctum::check().await {
    ///     println!("User is authenticated via Sanctum");
    /// }
    /// # }
    /// ```
    pub async fn check() -> bool {
        let manager = GLOBAL_SANCTUM.read().await;
        manager.check()
    }

    /// Get the currently authenticated user (requires LoadFromToken trait)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// // Returns None if not authenticated or user can't be loaded
    /// let user = Sanctum::user::<MyUser>().await;
    /// # }
    /// ```
    pub async fn user<T>() -> Option<T>
    where
        T: LoadFromToken + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;

        if let (Some(user_id), Some(db)) = (manager.current_user_id(), manager.database()) {
            T::load_from_token(user_id, &db).await.ok()
        } else {
            None
        }
    }

    /// Get the current access token
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// if let Some(token) = Sanctum::currentAccessToken().await {
    ///     println!("Token: {}", token.name);
    /// }
    /// # }
    /// ```
    pub async fn currentAccessToken() -> Option<PersonalAccessToken> {
        let manager = GLOBAL_SANCTUM.read().await;
        manager.current_token().cloned()
    }

    /// Check if the current token has a specific ability
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// if Sanctum::tokenCan("read:posts").await {
    ///     println!("User can read posts");
    /// }
    /// # }
    /// ```
    pub async fn tokenCan(ability: &str) -> bool {
        let manager = GLOBAL_SANCTUM.read().await;
        manager.token_can(ability)
    }

    /// Check if the current token has any of the abilities
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// if Sanctum::tokenCanAny(&["read:posts", "write:posts"]).await {
    ///     println!("User can read or write posts");
    /// }
    /// # }
    /// ```
    pub async fn tokenCanAny(abilities: &[&str]) -> bool {
        let manager = GLOBAL_SANCTUM.read().await;
        manager.token_can_any(abilities)
    }

    /// Check if the current token has all abilities
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() {
    /// if Sanctum::tokenCanAll(&["read:posts", "write:posts"]).await {
    ///     println!("User can both read and write posts");
    /// }
    /// # }
    /// ```
    pub async fn tokenCanAll(abilities: &[&str]) -> bool {
        let manager = GLOBAL_SANCTUM.read().await;
        manager.token_can_all(abilities)
    }

    /// Create a new token for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = Sanctum::createToken(
    ///     &user,
    ///     "mobile-app",
    ///     vec!["read:posts", "write:posts"],
    ///     None
    /// ).await?;
    /// println!("New token: {}", token.access_token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn createToken<T>(
        user: &T,
        name: &str,
        abilities: Vec<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<NewToken, SanctumError>
    where
        T: Tokenable + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        user.create_token(name, abilities, expires_at, &db).await
    }

    /// Create a token with device information
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = Sanctum::createTokenWithDevice(
    ///     &user,
    ///     "mobile-app",
    ///     vec!["*"],
    ///     None,
    ///     Some("Mozilla/5.0...".to_string()),
    ///     Some("192.168.1.1".to_string())
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn createTokenWithDevice<T>(
        user: &T,
        name: &str,
        abilities: Vec<&str>,
        expires_at: Option<DateTime<Utc>>,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<NewToken, SanctumError>
    where
        T: Tokenable + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        user.create_token_with_device(name, abilities, expires_at, user_agent, ip_address, &db)
            .await
    }

    /// Revoke the current access token
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Sanctum::revokeCurrentToken().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revokeCurrentToken() -> Result<(), SanctumError> {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        if let Some(token) = manager.current_token() {
            let repo = TokenRepository::new(&db);
            repo.revoke(token.id).await?;
            Ok(())
        } else {
            Err(SanctumError::InvalidToken)
        }
    }

    /// Revoke all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// Sanctum::revokeAllTokens(&user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revokeAllTokens<T>(user: &T) -> Result<(), SanctumError>
    where
        T: Tokenable + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        user.revoke_all_tokens(&db).await
    }

    /// Revoke a specific token by ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Sanctum::revokeToken(123).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revokeToken(token_id: i64) -> Result<(), SanctumError> {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        let repo = TokenRepository::new(&db);
        repo.revoke(token_id).await
    }

    /// Get all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let tokens = Sanctum::tokens(&user).await?;
    /// println!("User has {} tokens", tokens.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tokens<T>(user: &T) -> Result<Vec<PersonalAccessToken>, SanctumError>
    where
        T: Tokenable + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        user.tokens(&db).await
    }

    /// Prune expired tokens
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = Sanctum::pruneExpiredTokens().await?;
    /// println!("Deleted {} expired tokens", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pruneExpiredTokens() -> Result<u64, SanctumError> {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        let repo = TokenRepository::new(&db);
        repo.prune_expired_tokens().await
    }

    /// Prune tokens older than specified days
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = Sanctum::pruneTokensOlderThan(90).await?;
    /// println!("Deleted {} old tokens", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pruneTokensOlderThan(days: u32) -> Result<u64, SanctumError> {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        let repo = TokenRepository::new(&db);
        repo.prune_tokens_older_than(days).await
    }

    /// Prune unused tokens (not used in last N days)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let deleted = Sanctum::pruneUnusedTokens(30).await?;
    /// println!("Deleted {} unused tokens", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pruneUnusedTokens(days: u32) -> Result<u64, SanctumError> {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        let repo = TokenRepository::new(&db);
        repo.prune_unused_tokens(days).await
    }

    /// Get token statistics for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_sanctum_facade::Sanctum;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let stats = Sanctum::tokenStats(&user).await?;
    /// println!("Total tokens: {}, Active: {}", stats.total, stats.active);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tokenStats<T>(user: &T) -> Result<TokenStats, SanctumError>
    where
        T: Tokenable + Send + Sync,
    {
        let manager = GLOBAL_SANCTUM.read().await;
        let db = manager.database().ok_or(SanctumError::DatabaseNotConfigured)?;

        let repo = TokenRepository::new(&db);
        repo.get_token_stats(T::tokenable_type(), user.tokenable_id()).await
    }

    /// Set the current authentication context (called by middleware)
    ///
    /// This is typically called by authentication middleware and should not be
    /// called directly in application code.
    pub async fn setCurrentContext(
        token: PersonalAccessToken,
        user_id: i64,
        tokenable_type: String,
    ) {
        let mut manager = GLOBAL_SANCTUM.write().await;
        manager.set_current_token(token, user_id, tokenable_type);
    }

    /// Clear the current authentication context
    ///
    /// This is typically called at the end of a request or by middleware.
    pub async fn clearContext() {
        let mut manager = GLOBAL_SANCTUM.write().await;
        manager.clear_context();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sanctum_check_not_authenticated() {
        // Clear any existing context
        Sanctum::clearContext().await;
        assert!(!Sanctum::check().await);
    }

    #[tokio::test]
    async fn test_sanctum_token_can_not_authenticated() {
        Sanctum::clearContext().await;
        assert!(!Sanctum::tokenCan("read:posts").await);
    }

    #[tokio::test]
    async fn test_sanctum_static_methods_exist() {
        // Just verify methods compile and are callable
        let _ = Sanctum::check().await;
        let _ = Sanctum::currentAccessToken().await;
        let _ = Sanctum::tokenCan("test").await;
    }
}
