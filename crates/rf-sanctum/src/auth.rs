//! Sanctum authentication extractor for Axum

use crate::{
    config::SanctumConfig, repository::TokenRepository, transient::TransientTokenStore,
    PersonalAccessToken, SanctumError,
};
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use sea_orm::DatabaseConnection;

/// Extractor for Sanctum-authenticated users
///
/// # Example
///
/// ```rust,ignore
/// async fn protected(SanctumAuth(user): SanctumAuth<User>) -> Json<User> {
///     Json(user)
/// }
/// ```
pub struct SanctumAuth<T>(pub T, pub PersonalAccessToken);

/// Trait for loading a user from a token
#[async_trait]
pub trait LoadFromToken: Send + Sync + Sized {
    /// Load user by tokenable_id from the database (persistent path).
    async fn load_from_token(
        tokenable_id: i64,
        db: &DatabaseConnection,
    ) -> Result<Self, SanctumError>;

    /// Load user from a transient (in-memory) token without database access.
    ///
    /// Override this to support DB-free deployments, tests, or any scenario where
    /// [`SanctumConfig::allow_transient_tokens`] is `true` and a [`TransientTokenStore`]
    /// is present in request extensions instead of a [`DatabaseConnection`].
    ///
    /// The default implementation returns [`SanctumError::DatabaseNotConfigured`], which
    /// causes the extractor to fall through to `Unauthenticated` — i.e., transient auth
    /// is an opt-in: implement this method to enable it for your user type.
    async fn load_from_transient_token(
        _token: &PersonalAccessToken,
    ) -> Result<Self, SanctumError> {
        Err(SanctumError::DatabaseNotConfigured)
    }
}

impl<T, S> FromRequestParts<S> for SanctumAuth<T>
where
    T: LoadFromToken + 'static,
    S: Send + Sync,
{
    type Rejection = SanctumError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Extract bearer token from Authorization header first (cheap, fail-fast).
        let bearer_token = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(SanctumError::MissingToken)?
            .to_owned();

        // --- Database path (preferred when a DatabaseConnection is available) ---
        if let Some(db) = parts.extensions.get::<DatabaseConnection>() {
            // Hash the token to match database
            let hashed = PersonalAccessToken::hash_token(&bearer_token);

            // Find token in database
            let repo = TokenRepository::new(db);
            let token_model = repo
                .find_by_token(&hashed)
                .await?
                .ok_or(SanctumError::InvalidToken)?;

            // Check if token is expired
            if token_model.is_expired() {
                return Err(SanctumError::TokenExpired);
            }

            // Extract IP address for device tracking
            let ip = extract_client_ip(parts);

            // Update last_used_at and IP
            if ip.is_some() {
                repo.touch_with_ip(token_model.id, ip.clone()).await?;
            } else {
                repo.touch(token_model.id).await?;
            }

            // Load user
            let user = T::load_from_token(token_model.tokenable_id, db).await?;

            // Convert model to PersonalAccessToken
            let token = PersonalAccessToken::from_model(token_model);

            // Store token in extensions for middleware to access
            parts.extensions.insert(token.clone());

            return Ok(SanctumAuth(user, token));
        }

        // --- Transient path (no DatabaseConnection; uses in-memory TransientTokenStore) ---
        let config = parts
            .extensions
            .get::<SanctumConfig>()
            .cloned()
            .unwrap_or_default();

        if config.allow_transient_tokens {
            if let Some(store) = parts.extensions.get::<TransientTokenStore>().cloned() {
                let hashed = PersonalAccessToken::hash_token(&bearer_token);

                let token_data = store
                    .find(&hashed)?
                    .ok_or(SanctumError::InvalidToken)?;

                if token_data.is_expired() {
                    return Err(SanctumError::TokenExpired);
                }

                // Update last_used_at in the transient store
                store.touch(&hashed)?;

                // Load user via the transient (DB-free) path
                let user = T::load_from_transient_token(&token_data).await?;

                // Store token in extensions for downstream middleware
                parts.extensions.insert(token_data.clone());

                return Ok(SanctumAuth(user, token_data));
            }
        }

        Err(SanctumError::Unauthenticated)
    }
}

/// Extract client IP address from request headers
fn extract_client_ip(parts: &axum::http::request::Parts) -> Option<String> {
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

    /// Minimal in-test user type used to drive the transient auth path.
    struct TestUser {
        pub id: i64,
    }

    #[async_trait::async_trait]
    impl LoadFromToken for TestUser {
        async fn load_from_token(
            tokenable_id: i64,
            _db: &sea_orm::DatabaseConnection,
        ) -> Result<Self, SanctumError> {
            Ok(TestUser { id: tokenable_id })
        }

        /// DB-free override: construct the user from the token's tokenable_id.
        async fn load_from_transient_token(
            token: &PersonalAccessToken,
        ) -> Result<Self, SanctumError> {
            Ok(TestUser {
                id: token.tokenable_id,
            })
        }
    }

    // -----------------------------------------------------------------------
    // Transient auth: valid token, no DatabaseConnection → 200-equivalent
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_transient_auth_valid_token_no_db() {
        let store = TransientTokenStore::new();
        let (plain_token, token_data) = TransientTokenBuilder::new("User", 42, "api-key")
            .with_abilities(vec!["read:*".to_string()])
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

        let result = SanctumAuth::<TestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_ok(), "valid transient token must succeed: {:?}", result.err());
        let auth = result.unwrap();
        assert_eq!(auth.0.id, 42, "loaded user id must match token's tokenable_id");
    }

    // -----------------------------------------------------------------------
    // Transient auth: invalid / unknown bearer → InvalidToken
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_transient_auth_invalid_token_no_db() {
        let store = TransientTokenStore::new(); // empty — no tokens registered

        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer not_a_real_token_xyz")
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        parts.extensions.insert(store.clone());
        parts.extensions.insert(SanctumConfig::default());

        let result = SanctumAuth::<TestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, SanctumError::InvalidToken),
            "unknown bearer token must yield InvalidToken, got: {:?}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // Transient auth: missing Authorization header → MissingToken
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_transient_auth_missing_header_no_db() {
        let store = TransientTokenStore::new();

        let req = Request::builder().uri("/").body(()).unwrap();

        let (mut parts, _) = req.into_parts();
        parts.extensions.insert(store.clone());
        parts.extensions.insert(SanctumConfig::default());

        let result = SanctumAuth::<TestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, SanctumError::MissingToken),
            "absent Authorization header must yield MissingToken, got: {:?}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // Transient auth: expired token → TokenExpired
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_transient_auth_expired_token_no_db() {
        use chrono::{Duration, Utc};

        let store = TransientTokenStore::new();
        let (plain_token, token_data) = TransientTokenBuilder::new("User", 1, "old-key")
            .with_expiration(Utc::now() - Duration::hours(1))
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

        let result = SanctumAuth::<TestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, SanctumError::TokenExpired),
            "expired transient token must yield TokenExpired, got: {:?}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // No transient store and no DB → Unauthenticated
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_no_db_no_transient_store_yields_unauthenticated() {
        let req = Request::builder()
            .uri("/")
            .header("Authorization", "Bearer some_token")
            .body(())
            .unwrap();

        let (mut parts, _) = req.into_parts();
        // Neither DatabaseConnection nor TransientTokenStore in extensions

        let result = SanctumAuth::<TestUser>::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            matches!(err, SanctumError::Unauthenticated),
            "no auth backend must yield Unauthenticated, got: {:?}",
            err
        );
    }
}
