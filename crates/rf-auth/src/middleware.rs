//! Authentication middleware for Axum
//!
//! Provides middleware for protecting routes with JWT authentication.

use crate::auth_manager::with_auth_scope;
use crate::{jwt::JwtManager, Claims};
use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
};
use rf_core::error::AppError;
use std::sync::Arc;

/// Establishes a fresh per-request authentication scope.
///
/// Every request handled through this middleware gets its own isolated auth state
/// (current user, remember-me, guard), so a login performed while serving one
/// request can never leak into another concurrent request. Add it once near the
/// top of your middleware stack:
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use rf_auth::middleware::auth_scope;
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn(auth_scope));
/// ```
pub async fn auth_scope(req: Request, next: Next) -> Response {
    with_auth_scope(next.run(req)).await
}

/// Authentication middleware that validates JWT tokens
///
/// Extracts the JWT token from the Authorization header,
/// validates it, and adds the claims to request extensions.
///
/// # Example
///
/// ```ignore
/// use rf_auth::{JwtManager, middleware::auth_middleware};
/// use axum::{Router, routing::get, middleware as axum_middleware};
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("your-secret-key-min-32-characters")?);
///
/// let app = Router::new()
///     .route("/protected", get(protected_handler))
///     .layer(axum_middleware::from_fn(move |req, next| {
///         auth_middleware(req, next, jwt.clone())
///     }));
///
/// async fn protected_handler() -> &'static str {
///     "Protected content"
/// }
/// ```
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
    jwt_manager: Arc<JwtManager>,
) -> Result<Response, AppError> {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    // Validate token
    let claims = jwt_manager
        .validate_token(token)
        .map_err(|_| AppError::Unauthorized)?;

    // Add claims to request extensions
    req.extensions_mut().insert(claims);

    // Continue to handler
    Ok(next.run(req).await)
}

/// Create authentication middleware closure
///
/// Use this with `axum::Extension` to provide the JWT manager to routes.
///
/// # Example
///
/// ```ignore
/// use rf_auth::{JwtManager, middleware::auth_layer};
/// use axum::{Router, routing::get, Extension};
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("your-secret-key-min-32-characters")?);
///
/// let app = Router::new()
///     .route("/protected", get(protected_handler))
///     .layer(Extension(jwt.clone()))
///     .route_layer(axum::middleware::from_fn(auth_layer));
///
/// async fn protected_handler() -> &'static str {
///     "Protected content"
/// }
/// ```
pub async fn auth_layer(
    Extension(jwt): Extension<Arc<JwtManager>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    auth_middleware(req, next, jwt).await
}

/// Check if user has a specific role (use after auth_layer)
///
/// # Example
///
/// ```no_run
/// use rf_auth::{Claims, middleware::require_role};
/// use axum::http::StatusCode;
///
/// async fn admin_handler(claims: Claims) -> Result<&'static str, (StatusCode, String)> {
///     require_role(&claims, "admin")?;
///     Ok("Admin content")
/// }
/// ```
pub fn require_role(claims: &Claims, role: &str) -> Result<(), (axum::http::StatusCode, String)> {
    if !claims.has_role(role) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            format!("Role '{}' required", role),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Claims;

    const TEST_SECRET: &str = "test-secret-key-min-32-characters-long";

    #[tokio::test]
    async fn test_require_role_success() {
        let claims = Claims::new(
            1,
            "test@example.com".into(),
            vec!["user".into(), "admin".into()],
            1,
        );

        assert!(require_role(&claims, "user").is_ok());
        assert!(require_role(&claims, "admin").is_ok());
    }

    #[tokio::test]
    async fn test_require_role_failure() {
        let claims = Claims::new(1, "test@example.com".into(), vec!["user".into()], 1);

        let result = require_role(&claims, "admin");
        assert!(result.is_err());

        if let Err((status, msg)) = result {
            assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
            assert!(msg.contains("admin"));
        }
    }
}
