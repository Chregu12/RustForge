//! Passport facade providing Laravel-style static API for OAuth2

use crate::config::{GrantControl, PkceControl, TokenLifetimes};
use crate::manager::GLOBAL_PASSPORT;
use async_trait::async_trait;
use chrono::Duration;
use rf_passport::{
    ClientRepository, HasApiTokens, OAuthAccessToken, OAuthClient, PassportConfig, PassportError,
    PassportResult, Scope, TokenRepository,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// The Passport facade providing a static-like API for OAuth2 server.
///
/// This is the main entry point for Passport OAuth2 management in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_passport_facade::Passport;
/// use chrono::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Configure token lifetimes
/// Passport::tokensExpireIn(Duration::seconds(3600 * 24 * 15)).await;
///
/// // Define scopes
/// Passport::tokensCan(&[
///     ("read:posts", "Read blog posts"),
///     ("write:posts", "Create and edit posts"),
/// ]).await;
/// # Ok(())
/// # }
/// ```
pub struct Passport;

impl Passport {
    /// Configure Passport with a database connection
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use sea_orm::DatabaseConnection;
    /// use std::sync::Arc;
    ///
    /// # async fn example(db: DatabaseConnection) {
    /// Passport::setDatabase(Arc::new(db)).await;
    /// # }
    /// ```
    pub async fn setDatabase(db: Arc<DatabaseConnection>) {
        let mut manager = GLOBAL_PASSPORT.write().await;
        manager.set_database(db);
    }

    /// Get the current configuration
    pub async fn config() -> PassportConfig {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.config().clone()
    }

    // ===== Token Lifetime Configuration =====

    /// Set access token lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # async fn example() {
    /// Passport::tokensExpireIn(Duration::seconds(3600 * 24 * 15)).await;
    /// # }
    /// ```
    pub async fn tokensExpireIn(duration: Duration) {
        TokenLifetimes::access_tokens_expire_in(duration).await;
    }

    /// Set refresh token lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # async fn example() {
    /// Passport::refreshTokensExpireIn(Duration::seconds(3600 * 24 * 30)).await;
    /// # }
    /// ```
    pub async fn refreshTokensExpireIn(duration: Duration) {
        TokenLifetimes::refresh_tokens_expire_in(duration).await;
    }

    /// Set personal access token lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # async fn example() {
    /// Passport::personalAccessTokensExpireIn(Duration::seconds(3600 * 24 * 365)).await;
    /// # }
    /// ```
    pub async fn personalAccessTokensExpireIn(duration: Duration) {
        TokenLifetimes::personal_access_tokens_expire_in(duration).await;
    }

    /// Set authorization code lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # async fn example() {
    /// Passport::authCodesExpireIn(Duration::seconds(600)).await;
    /// # }
    /// ```
    pub async fn authCodesExpireIn(duration: Duration) {
        TokenLifetimes::auth_codes_expire_in(duration).await;
    }

    // ===== Scope Management =====

    /// Define OAuth scopes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// Passport::tokensCan(&[
    ///     ("read:posts", "Read blog posts"),
    ///     ("write:posts", "Create and edit posts"),
    ///     ("delete:posts", "Delete posts"),
    /// ]).await;
    /// # }
    /// ```
    pub async fn tokensCan(scopes: &[(&str, &str)]) {
        let mut manager = GLOBAL_PASSPORT.write().await;
        let scope_list: Vec<Scope> = scopes
            .iter()
            .map(|(id, desc)| Scope::new(*id, *desc))
            .collect();
        manager.register_scopes(scope_list);
    }

    /// Set default scopes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// Passport::setDefaultScope(&["read:posts"]).await;
    /// # }
    /// ```
    pub async fn setDefaultScope(scopes: &[&str]) {
        let mut manager = GLOBAL_PASSPORT.write().await;
        manager.set_default_scopes(scopes.iter().map(|s| s.to_string()).collect());
    }

    /// Check if a scope is valid/registered
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// if Passport::hasScope("read:posts").await {
    ///     println!("Scope is registered");
    /// }
    /// # }
    /// ```
    pub async fn hasScope(scope: &str) -> bool {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.has_scope(scope)
    }

    /// Get all registered scopes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// let scopes = Passport::scopes().await;
    /// for scope in scopes {
    ///     println!("{}: {}", scope.id, scope.description);
    /// }
    /// # }
    /// ```
    pub async fn scopes() -> Vec<Scope> {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.all_scopes().iter().map(|s| (*s).clone()).collect()
    }

    /// Get a specific scope
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// if let Some(scope) = Passport::scope("read:posts").await {
    ///     println!("Description: {}", scope.description);
    /// }
    /// # }
    /// ```
    pub async fn scope(scope_id: &str) -> Option<Scope> {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.get_scope(scope_id).cloned()
    }

    // ===== Personal Access Tokens =====

    /// Create a personal access token for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = Passport::createToken(&user, "api-token", vec!["read:posts".to_string()]).await?;
    /// println!("Token created: {}", token);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn createToken<T>(
        user: &T,
        name: &str,
        scopes: Vec<String>,
    ) -> PassportResult<String>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        let config = manager.config().clone();
        drop(manager);

        user.create_token(name, scopes, &db, &config).await
    }

    /// Get all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let tokens = Passport::tokens(&user).await?;
    /// println!("User has {} tokens", tokens.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tokens<T>(user: &T) -> PassportResult<Vec<OAuthAccessToken>>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        user.tokens(&db).await
    }

    /// Revoke a specific token
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Passport::revokeToken("token_id").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revokeToken(token_id: &str) -> PassportResult<()> {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        let repo = TokenRepository::new(&db);
        repo.revoke_access_token(token_id).await
    }

    /// Revoke all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let count = Passport::revokeAllTokens(&user).await?;
    /// println!("Revoked {} tokens", count);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn revokeAllTokens<T>(user: &T) -> PassportResult<u64>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        user.revoke_all_tokens(&db).await
    }

    // ===== Token Abilities (for current request) =====

    /// Check if the current token has a specific scope
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// if Passport::tokenCan("write:posts").await {
    ///     println!("User can write posts");
    /// }
    /// # }
    /// ```
    pub async fn tokenCan(scope: &str) -> bool {
        let manager = GLOBAL_PASSPORT.read().await;
        if let Some(token_id) = manager.current_token_id() {
            if let Some(db) = manager.database() {
                let repo = TokenRepository::new(&db);
                if let Ok(Some(token)) = repo.find_access_token(token_id).await {
                    return token.has_scope(scope);
                }
            }
        }
        false
    }

    /// Get the authenticated user via Passport
    ///
    /// Returns the user ID if authenticated via Passport
    pub async fn userId() -> Option<i64> {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.current_user_id()
    }

    /// Check if authenticated via Passport
    pub async fn check() -> bool {
        let manager = GLOBAL_PASSPORT.read().await;
        manager.check()
    }

    // ===== Client Management =====

    /// Create an OAuth client
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Passport::createClient(
    ///     "My App",
    ///     "https://app.com/callback"
    /// ).await?;
    /// println!("Client ID: {}", client.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn createClient(name: &str, redirect: &str) -> PassportResult<(OAuthClient, Option<String>)> {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        let repo = ClientRepository::new(&db);
        repo.create(None, name, vec![redirect.to_string()], false, false, true).await
    }

    /// Get all clients for a user
    pub async fn clients(user_id: i64) -> PassportResult<Vec<OAuthClient>> {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        let repo = ClientRepository::new(&db);
        repo.find_by_user(user_id).await
    }

    /// Delete a client
    pub async fn deleteClient(client_id: i64) -> PassportResult<()> {
        let manager = GLOBAL_PASSPORT.read().await;
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        let repo = ClientRepository::new(&db);
        repo.delete(client_id).await
    }

    // ===== Grant Control =====

    /// Enable password grant
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// Passport::enablePasswordGrant().await;
    /// # }
    /// ```
    pub async fn enablePasswordGrant() {
        GrantControl::enable_password_grant().await;
    }

    /// Disable password grant
    pub async fn disablePasswordGrant() {
        GrantControl::disable_password_grant().await;
    }

    /// Enable implicit grant
    pub async fn enableImplicitGrant() {
        GrantControl::enable_implicit_grant().await;
    }

    /// Disable implicit grant
    pub async fn disableImplicitGrant() {
        GrantControl::disable_implicit_grant().await;
    }

    /// Enable client credentials grant
    pub async fn enableClientCredentialsGrant() {
        GrantControl::enable_client_credentials_grant().await;
    }

    /// Disable client credentials grant
    pub async fn disableClientCredentialsGrant() {
        GrantControl::disable_client_credentials_grant().await;
    }

    /// Enable authorization code grant
    pub async fn enableAuthorizationCodeGrant() {
        GrantControl::enable_authorization_code_grant().await;
    }

    /// Disable authorization code grant
    pub async fn disableAuthorizationCodeGrant() {
        GrantControl::disable_authorization_code_grant().await;
    }

    // ===== PKCE Control =====

    /// Require PKCE for authorization code flow
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # async fn example() {
    /// Passport::requirePkce(true).await;
    /// # }
    /// ```
    pub async fn requirePkce(enforce: bool) {
        PkceControl::require_pkce(enforce).await;
    }

    /// Allow plain text PKCE (not recommended)
    pub async fn allowPlainPkce(allow: bool) {
        PkceControl::allow_plain_pkce(allow).await;
    }

    // ===== Context Management (for middleware) =====

    /// Set the current authentication context (called by middleware)
    pub async fn setCurrentContext(token_id: String, user_id: i64) {
        let mut manager = GLOBAL_PASSPORT.write().await;
        manager.set_current_token(token_id, user_id);
    }

    /// Clear the current authentication context
    pub async fn clearContext() {
        let mut manager = GLOBAL_PASSPORT.write().await;
        manager.clear_context();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_passport_check_not_authenticated() {
        Passport::clearContext().await;
        assert!(!Passport::check().await);
    }

    #[tokio::test]
    async fn test_passport_scope_management() {
        Passport::tokensCan(&[
            ("read:posts", "Read posts"),
            ("write:posts", "Write posts"),
        ])
        .await;

        assert!(Passport::hasScope("read:posts").await);
        assert!(Passport::hasScope("write:posts").await);
        assert!(!Passport::hasScope("delete:posts").await);
    }

    #[tokio::test]
    async fn test_passport_default_scopes() {
        Passport::setDefaultScope(&["read:posts"]).await;
        let manager = GLOBAL_PASSPORT.read().await;
        assert_eq!(manager.default_scopes(), &["read:posts"]);
    }

    #[tokio::test]
    async fn test_passport_static_methods_exist() {
        // Just verify methods compile and are callable
        let _ = Passport::check().await;
        let _ = Passport::userId().await;
        let _ = Passport::scopes().await;
    }
}
