//! Passport facade providing Laravel-style static API for OAuth2

// The facade methods intentionally use Laravel's camelCase names (e.g. `createToken`,
// `tokensExpireIn`) as part of the public API, so relax the Rust naming lint here only.
#![allow(non_snake_case)]

use crate::config::{GrantControl, PkceControl, TokenLifetimes};
use crate::manager::GLOBAL_PASSPORT;
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
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Configure token lifetimes
/// Passport::tokensExpireIn(Duration::seconds(3600 * 24 * 15));
///
/// // Define scopes
/// Passport::tokensCan(&[
///     ("read:posts", "Read blog posts"),
///     ("write:posts", "Create and edit posts"),
/// ]);
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
    /// # fn example(db: DatabaseConnection) {
    /// Passport::setDatabase(Arc::new(db));
    /// # }
    /// ```
    pub fn setDatabase(db: Arc<DatabaseConnection>) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.set_database(db);
    }

    /// Get the current configuration
    pub fn config() -> PassportConfig {
        let manager = GLOBAL_PASSPORT.read().unwrap();
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
    /// # fn example() {
    /// Passport::tokensExpireIn(Duration::seconds(3600 * 24 * 15));
    /// # }
    /// ```
    pub fn tokensExpireIn(duration: Duration) {
        TokenLifetimes::access_tokens_expire_in(duration);
    }

    /// Set refresh token lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # fn example() {
    /// Passport::refreshTokensExpireIn(Duration::seconds(3600 * 24 * 30));
    /// # }
    /// ```
    pub fn refreshTokensExpireIn(duration: Duration) {
        TokenLifetimes::refresh_tokens_expire_in(duration);
    }

    /// Set personal access token lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # fn example() {
    /// Passport::personalAccessTokensExpireIn(Duration::seconds(3600 * 24 * 365));
    /// # }
    /// ```
    pub fn personalAccessTokensExpireIn(duration: Duration) {
        TokenLifetimes::personal_access_tokens_expire_in(duration);
    }

    /// Set authorization code lifetime
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// use chrono::Duration;
    ///
    /// # fn example() {
    /// Passport::authCodesExpireIn(Duration::seconds(600));
    /// # }
    /// ```
    pub fn authCodesExpireIn(duration: Duration) {
        TokenLifetimes::auth_codes_expire_in(duration);
    }

    // ===== Scope Management =====

    /// Define OAuth scopes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// Passport::tokensCan(&[
    ///     ("read:posts", "Read blog posts"),
    ///     ("write:posts", "Create and edit posts"),
    ///     ("delete:posts", "Delete posts"),
    /// ]);
    /// # }
    /// ```
    pub fn tokensCan(scopes: &[(&str, &str)]) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
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
    /// # fn example() {
    /// Passport::setDefaultScope(&["read:posts"]);
    /// # }
    /// ```
    pub fn setDefaultScope(scopes: &[&str]) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.set_default_scopes(scopes.iter().map(|s| s.to_string()).collect());
    }

    /// Check if a scope is valid/registered
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// if Passport::hasScope("read:posts") {
    ///     println!("Scope is registered");
    /// }
    /// # }
    /// ```
    pub fn hasScope(scope: &str) -> bool {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        manager.has_scope(scope)
    }

    /// Get all registered scopes
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// let scopes = Passport::scopes();
    /// for scope in scopes {
    ///     println!("{}: {}", scope.id, scope.description);
    /// }
    /// # }
    /// ```
    pub fn scopes() -> Vec<Scope> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        manager.all_scopes().iter().map(|s| (*s).clone()).collect()
    }

    /// Get a specific scope
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// if let Some(scope) = Passport::scope("read:posts") {
    ///     println!("Description: {}", scope.description);
    /// }
    /// # }
    /// ```
    pub fn scope(scope_id: &str) -> Option<Scope> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        manager.get_scope(scope_id).cloned()
    }

    // ===== Personal Access Tokens =====

    /// Create a personal access token for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// # use rf_passport::HasApiTokens;
    /// # struct MyUser { id: i64 }
    /// # impl HasApiTokens for MyUser {
    /// #     fn get_id(&self) -> i64 { self.id }
    /// # }
    ///
    /// # fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let token = Passport::createToken(&user, "api-token", vec!["read:posts".to_string()])?;
    /// println!("Token created: {}", token);
    /// # Ok(())
    /// # }
    /// ```
    pub fn createToken<T>(
        user: &T,
        name: &str,
        scopes: Vec<String>,
    ) -> PassportResult<String>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        let config = manager.config().clone();
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                user.create_token(name, scopes, &db, &config).await
            })
        })
    }

    /// Get all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// # use rf_passport::HasApiTokens;
    /// # struct MyUser { id: i64 }
    /// # impl HasApiTokens for MyUser {
    /// #     fn get_id(&self) -> i64 { self.id }
    /// # }
    ///
    /// # fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let tokens = Passport::tokens(&user)?;
    /// println!("User has {} tokens", tokens.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn tokens<T>(user: &T) -> PassportResult<Vec<OAuthAccessToken>>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                user.tokens(&db).await
            })
        })
    }

    /// Revoke a specific token
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Passport::revokeToken("token_id")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn revokeToken(token_id: &str) -> PassportResult<()> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let repo = TokenRepository::new(&db);
                repo.revoke_access_token(token_id).await
            })
        })
    }

    /// Revoke all tokens for a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    /// # use rf_passport::HasApiTokens;
    /// # struct MyUser { id: i64 }
    /// # impl HasApiTokens for MyUser {
    /// #     fn get_id(&self) -> i64 { self.id }
    /// # }
    ///
    /// # fn example(user: MyUser) -> Result<(), Box<dyn std::error::Error>> {
    /// let count = Passport::revokeAllTokens(&user)?;
    /// println!("Revoked {} tokens", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn revokeAllTokens<T>(user: &T) -> PassportResult<u64>
    where
        T: HasApiTokens + Send + Sync,
    {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                user.revoke_all_tokens(&db).await
            })
        })
    }

    // ===== Token Abilities (for current request) =====

    /// Check if the current token has a specific scope
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// if Passport::tokenCan("write:posts") {
    ///     println!("User can write posts");
    /// }
    /// # }
    /// ```
    pub fn tokenCan(scope: &str) -> bool {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        if let Some(token_id) = manager.current_token_id() {
            if let Some(db) = manager.database() {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        let repo = TokenRepository::new(&db);
                        if let Ok(Some(token)) = repo.find_access_token(token_id).await {
                            return token.has_scope(scope);
                        }
                        false
                    })
                })
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get the authenticated user via Passport
    ///
    /// Returns the user ID if authenticated via Passport
    pub fn userId() -> Option<i64> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        manager.current_user_id()
    }

    /// Check if authenticated via Passport
    pub fn check() -> bool {
        let manager = GLOBAL_PASSPORT.read().unwrap();
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
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Passport::createClient(
    ///     "My App",
    ///     "https://app.com/callback"
    /// )?;
    /// println!("Client ID: {}", client.0.id);
    /// # Ok(())
    /// # }
    /// ```
    pub fn createClient(name: &str, redirect: &str) -> PassportResult<(OAuthClient, Option<String>)> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let repo = ClientRepository::new(&db);
                repo.create(None, name, vec![redirect.to_string()], false, false, true).await
            })
        })
    }

    /// Get all clients for a user
    pub fn clients(user_id: i64) -> PassportResult<Vec<OAuthClient>> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let repo = ClientRepository::new(&db);
                repo.find_by_user(user_id).await
            })
        })
    }

    /// Delete a client
    pub fn deleteClient(client_id: i64) -> PassportResult<()> {
        let manager = GLOBAL_PASSPORT.read().unwrap();
        let db = manager.database().ok_or(PassportError::ConfigurationError(
            "Database not configured".to_string(),
        ))?;
        drop(manager);

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let repo = ClientRepository::new(&db);
                repo.delete(client_id).await
            })
        })
    }

    // ===== Grant Control =====

    /// Enable password grant
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// Passport::enablePasswordGrant();
    /// # }
    /// ```
    pub fn enablePasswordGrant() {
        GrantControl::enable_password_grant();
    }

    /// Disable password grant
    pub fn disablePasswordGrant() {
        GrantControl::disable_password_grant();
    }

    /// Enable implicit grant
    pub fn enableImplicitGrant() {
        GrantControl::enable_implicit_grant();
    }

    /// Disable implicit grant
    pub fn disableImplicitGrant() {
        GrantControl::disable_implicit_grant();
    }

    /// Enable client credentials grant
    pub fn enableClientCredentialsGrant() {
        GrantControl::enable_client_credentials_grant();
    }

    /// Disable client credentials grant
    pub fn disableClientCredentialsGrant() {
        GrantControl::disable_client_credentials_grant();
    }

    /// Enable authorization code grant
    pub fn enableAuthorizationCodeGrant() {
        GrantControl::enable_authorization_code_grant();
    }

    /// Disable authorization code grant
    pub fn disableAuthorizationCodeGrant() {
        GrantControl::disable_authorization_code_grant();
    }

    // ===== PKCE Control =====

    /// Require PKCE for authorization code flow
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_passport_facade::Passport;
    ///
    /// # fn example() {
    /// Passport::requirePkce(true);
    /// # }
    /// ```
    pub fn requirePkce(enforce: bool) {
        PkceControl::require_pkce(enforce);
    }

    /// Allow plain text PKCE (not recommended)
    pub fn allowPlainPkce(allow: bool) {
        PkceControl::allow_plain_pkce(allow);
    }

    // ===== Context Management (for middleware) =====

    /// Set the current authentication context (called by middleware)
    pub fn setCurrentContext(token_id: String, user_id: i64) {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.set_current_token(token_id, user_id);
    }

    /// Clear the current authentication context
    pub fn clearContext() {
        let mut manager = GLOBAL_PASSPORT.write().unwrap();
        manager.clear_context();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passport_check_not_authenticated() {
        Passport::clearContext();
        assert!(!Passport::check());
    }

    #[test]
    fn test_passport_scope_management() {
        Passport::tokensCan(&[
            ("read:posts", "Read posts"),
            ("write:posts", "Write posts"),
        ]);

        assert!(Passport::hasScope("read:posts"));
        assert!(Passport::hasScope("write:posts"));
        assert!(!Passport::hasScope("delete:posts"));
    }

    #[test]
    fn test_passport_default_scopes() {
        Passport::setDefaultScope(&["read:posts"]);
        let manager = GLOBAL_PASSPORT.read().unwrap();
        assert_eq!(manager.default_scopes(), &["read:posts"]);
    }

    #[test]
    fn test_passport_static_methods_exist() {
        // Just verify methods compile and are callable
        let _ = Passport::check();
        let _ = Passport::userId();
        let _ = Passport::scopes();
    }
}
