//! Sanctum Guard for authentication
//!
//! Provides a unified interface for both API token authentication
//! and SPA cookie-based authentication.

use crate::{
    config::SanctumConfig, repository::TokenRepository, transient::TransientTokenStore,
    LoadFromToken, PersonalAccessToken, SanctumError,
};
use axum::{extract::FromRequestParts, http::request::Parts};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Sanctum Guard for authenticating requests
///
/// Supports both:
/// - API Token authentication (Bearer tokens)
/// - SPA Cookie authentication (session-based)
///
/// # Example
///
/// ```rust,ignore
/// use rf_sanctum::{SanctumGuard, LoadFromToken};
///
/// async fn protected(guard: SanctumGuard<User>) -> Json<User> {
///     Json(guard.user)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SanctumGuard<T> {
    /// The authenticated user
    pub user: T,
    /// The access token (if using API token auth)
    pub token: Option<PersonalAccessToken>,
    /// Authentication method used
    pub auth_method: AuthMethod,
}

/// Authentication method used
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// Bearer token authentication
    Token,
    /// SPA cookie/session authentication
    Cookie,
}

impl<T> SanctumGuard<T> {
    /// Check if the guard has a specific ability
    pub fn can(&self, ability: &str) -> bool {
        match &self.token {
            Some(token) => token.can(ability),
            None => false, // SPA auth has no token abilities
        }
    }

    /// Check if the guard has any of the abilities
    pub fn can_any(&self, abilities: &[&str]) -> bool {
        match &self.token {
            Some(token) => token.can_any(abilities),
            None => false,
        }
    }

    /// Check if the guard has all abilities
    pub fn can_all(&self, abilities: &[&str]) -> bool {
        match &self.token {
            Some(token) => token.can_all(abilities),
            None => false,
        }
    }

    /// Get the current access token (if using token auth)
    pub fn current_access_token(&self) -> Option<&PersonalAccessToken> {
        self.token.as_ref()
    }

    /// Check if authenticated via token
    pub fn is_token_auth(&self) -> bool {
        self.auth_method == AuthMethod::Token
    }

    /// Check if authenticated via cookie/session
    pub fn is_cookie_auth(&self) -> bool {
        self.auth_method == AuthMethod::Cookie
    }
}

impl<T, S> FromRequestParts<S> for SanctumGuard<T>
where
    T: LoadFromToken + 'static,
    S: Send + Sync,
{
    type Rejection = SanctumError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Get config from extensions (optional, use default if not provided)
        let config = parts
            .extensions
            .get::<SanctumConfig>()
            .cloned()
            .unwrap_or_default();

        // Clone the Arc<DatabaseConnection> if present (cheap clone of Arc pointer)
        let db_opt = parts
            .extensions
            .get::<Arc<DatabaseConnection>>()
            .cloned();

        // --- Bearer token path ---
        if let Some(bearer_token) = extract_bearer_token(parts) {
            // Prefer database path when a connection is available
            if let Some(ref db) = db_opt {
                return authenticate_via_token(bearer_token, &**db, &config, parts).await;
            }

            // Fall back to transient store (DB-free / test deployments)
            if config.allow_transient_tokens {
                if let Some(store) = parts.extensions.get::<TransientTokenStore>().cloned() {
                    return authenticate_via_transient_token(
                        bearer_token,
                        &store,
                        &config,
                        parts,
                    )
                    .await;
                }
            }

            // Bearer token present but no auth backend is configured
            return Err(SanctumError::Unauthenticated);
        }

        // --- SPA cookie / session path (requires database) ---
        if let Some(ref db) = db_opt {
            if let Some(user_id) = extract_session_user_id(parts) {
                return authenticate_via_cookie(user_id, &**db).await;
            }
        }

        Err(SanctumError::Unauthenticated)
    }
}

/// Extract bearer token from Authorization header
fn extract_bearer_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

/// Extract user ID from session cookie
fn extract_session_user_id(parts: &Parts) -> Option<i64> {
    // This is a placeholder - actual implementation would:
    // 1. Parse session cookie
    // 2. Decrypt/verify session data
    // 3. Extract user ID from session
    // For now, we'll check for a simple user_id extension
    parts.extensions.get::<i64>().copied()
}

/// Authenticate via API token
async fn authenticate_via_token<T>(
    bearer_token: String,
    db: &DatabaseConnection,
    config: &SanctumConfig,
    parts: &mut Parts,
) -> Result<SanctumGuard<T>, SanctumError>
where
    T: LoadFromToken,
{
    // Strip prefix if configured
    let token = config.strip_prefix(&bearer_token);

    // Hash the token
    let hashed = PersonalAccessToken::hash_token(token);

    // Find token in database
    let repo = TokenRepository::new(db);
    let token_model = repo
        .find_by_token(&hashed)
        .await?
        .ok_or(SanctumError::InvalidToken)?;

    // Check if expired
    if token_model.is_expired() {
        return Err(SanctumError::TokenExpired);
    }

    // Extract IP address if device tracking is enabled
    let ip = if config.track_devices {
        extract_client_ip(parts)
    } else {
        None
    };

    // Update last_used_at and IP
    if config.track_devices && ip.is_some() {
        token_model.touch_with_ip(db, ip.clone()).await.ok();
    } else {
        token_model.touch(db).await.ok();
    }

    // Load user
    let user = T::load_from_token(token_model.tokenable_id, db).await?;

    // Convert to PersonalAccessToken
    let token = PersonalAccessToken::from_model(token_model.clone());

    // Store in extensions for middleware
    parts.extensions.insert(token.clone());

    Ok(SanctumGuard {
        user,
        token: Some(token),
        auth_method: AuthMethod::Token,
    })
}

/// Authenticate via SPA cookie/session
async fn authenticate_via_cookie<T>(
    user_id: i64,
    db: &DatabaseConnection,
) -> Result<SanctumGuard<T>, SanctumError>
where
    T: LoadFromToken,
{
    // Load user from session
    let user = T::load_from_token(user_id, db).await?;

    Ok(SanctumGuard {
        user,
        token: None,
        auth_method: AuthMethod::Cookie,
    })
}

/// Authenticate via in-memory TransientTokenStore (no database required)
async fn authenticate_via_transient_token<T>(
    bearer_token: String,
    store: &TransientTokenStore,
    config: &SanctumConfig,
    parts: &mut Parts,
) -> Result<SanctumGuard<T>, SanctumError>
where
    T: LoadFromToken,
{
    // Strip prefix if configured
    let raw = config.strip_prefix(&bearer_token);

    // Hash the token to look it up in the store
    let hashed = PersonalAccessToken::hash_token(raw);

    // Find token in the transient store
    let token_data = store
        .find(&hashed)?
        .ok_or(SanctumError::InvalidToken)?;

    // Check expiry
    if token_data.is_expired() {
        return Err(SanctumError::TokenExpired);
    }

    // Update last_used_at in the transient store
    store.touch(&hashed)?;

    // Load user via the transient (DB-free) hook on LoadFromToken
    let user = T::load_from_transient_token(&token_data).await?;

    // Store token in extensions for downstream middleware
    parts.extensions.insert(token_data.clone());

    Ok(SanctumGuard {
        user,
        token: Some(token_data),
        auth_method: AuthMethod::Token,
    })
}

/// Extract client IP address from request
fn extract_client_ip(parts: &Parts) -> Option<String> {
    // Try X-Forwarded-For first (proxy/load balancer)
    if let Some(xff) = parts.headers.get("X-Forwarded-For") {
        if let Ok(value) = xff.to_str() {
            // Take the first IP in the list
            if let Some(ip) = value.split(',').next() {
                return Some(ip.trim().to_string());
            }
        }
    }

    // Try X-Real-IP
    if let Some(xri) = parts.headers.get("X-Real-IP") {
        if let Ok(value) = xri.to_str() {
            return Some(value.to_string());
        }
    }

    // Try to get from connection info (requires extension)
    parts
        .extensions
        .get::<std::net::SocketAddr>()
        .map(|addr| addr.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::SanctumConfig,
        transient::{TransientTokenBuilder, TransientTokenStore},
    };
    use axum::http::Request;

    #[test]
    fn test_auth_method() {
        assert_eq!(AuthMethod::Token, AuthMethod::Token);
        assert_ne!(AuthMethod::Token, AuthMethod::Cookie);
    }

    #[test]
    fn test_extract_bearer_token() {
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer test_token_123")
            .body(())
            .unwrap();

        let (parts, _) = req.into_parts();

        let token = extract_bearer_token(&parts);
        assert_eq!(token, Some("test_token_123".to_string()));
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let req = Request::builder().uri("/").body(()).unwrap();

        let (parts, _) = req.into_parts();

        let token = extract_bearer_token(&parts);
        assert_eq!(token, None);
    }

    // -----------------------------------------------------------------------
    // Minimal user type for transient guard tests
    // -----------------------------------------------------------------------
    struct GuardTestUser {
        pub id: i64,
    }

    #[async_trait::async_trait]
    impl LoadFromToken for GuardTestUser {
        async fn load_from_token(
            tokenable_id: i64,
            _db: &sea_orm::DatabaseConnection,
        ) -> Result<Self, SanctumError> {
            Ok(GuardTestUser { id: tokenable_id })
        }

        async fn load_from_transient_token(
            token: &PersonalAccessToken,
        ) -> Result<Self, SanctumError> {
            Ok(GuardTestUser {
                id: token.tokenable_id,
            })
        }
    }

    // -----------------------------------------------------------------------
    // SanctumGuard: valid transient token, no DatabaseConnection → success
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_guard_transient_valid_token_no_db() {
        let store = TransientTokenStore::new();
        let (plain_token, token_data) = TransientTokenBuilder::new("User", 99, "guard-key")
            .with_abilities(vec!["*".to_string()])
            .build();
        store.store(token_data).unwrap();

        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", plain_token))
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        parts.extensions.insert(store.clone());
        parts.extensions.insert(SanctumConfig::default());

        let result =
            SanctumGuard::<GuardTestUser>::from_request_parts(&mut parts, &()).await;
        assert!(
            result.is_ok(),
            "valid transient bearer must succeed: {:?}",
            result.err()
        );
        let guard = result.unwrap();
        assert_eq!(guard.user.id, 99);
        assert_eq!(guard.auth_method, AuthMethod::Token);
        assert!(guard.token.is_some());
    }

    // -----------------------------------------------------------------------
    // SanctumGuard: invalid bearer, no DB → InvalidToken
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_guard_transient_invalid_token_no_db() {
        let store = TransientTokenStore::new();

        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer totally_wrong")
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        parts.extensions.insert(store.clone());
        parts.extensions.insert(SanctumConfig::default());

        let result =
            SanctumGuard::<GuardTestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, SanctumError::InvalidToken),
            "unknown bearer must yield InvalidToken, got: {:?}",
            err
        );
    }
}
