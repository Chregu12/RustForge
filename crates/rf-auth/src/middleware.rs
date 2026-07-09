//! Authentication middleware for Axum
//!
//! Provides middleware for protecting routes with JWT authentication.

use crate::auth_manager::{with_auth_scope, AuthManager};
use crate::{jwt::JwtManager, Claims};
use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::{IntoResponse, Response},
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

/// Ready-made bearer-auth guard: rejects unauthenticated requests with a JSON 401
/// **before** the handler or any of its body extractors run.
///
/// This is the reusable, first-class replacement for the auth-scope + bearer→login
/// layer that apps used to hand-write per route. It:
///
/// 1. reads the `Authorization: Bearer <user_id>` header — **headers only, never
///    the body**;
/// 2. opens a fresh per-request auth scope (like [`auth_scope`]) so the downstream
///    handler's [`Auth`](crate::Auth) reads are isolated;
/// 3. **verifies** the id against the configured [`UserProvider`](crate::UserProvider)
///    via [`AuthManager::login_using_id_verified`] — an *existing* user only, so a
///    phantom id is never authorized;
/// 4. on success, establishes the authenticated user in the scope and runs the
///    handler (so it can read `Auth::check()` / `Auth::user()` / `Auth::id()`);
/// 5. on a missing, malformed, or non-existent-user token, short-circuits with
///    [`AppError::Unauthorized`] (401, framework JSON envelope) **without reading the
///    request body**.
///
/// Because the body is never consumed on the unauthenticated path, a guest posting
/// an *invalid* body still gets a 401 — auth wins over the 422 a body validator
/// (e.g. `ValidatedJson`) would otherwise raise during handler dispatch.
///
/// Register your provider once at startup with `Auth::set_provider(..)`, then guard
/// protected routes with this as a `route_layer`:
///
/// ```ignore
/// use axum::{Router, routing::post, middleware};
/// use rf_auth::require_auth;
///
/// let app = Router::new().route(
///     "/tasks",
///     post(create_task).route_layer(middleware::from_fn(require_auth)),
/// );
/// ```
pub async fn require_auth(req: Request, next: Next) -> Response {
    // Read the bearer id from headers ONLY — never touch the body, so this 401
    // precedes any body-extractor 422 further down the stack.
    let bearer_id = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .and_then(|t| t.trim().parse::<u64>().ok());

    with_auth_scope(async move {
        // Verifying login: only a bearer id that resolves to a real user via the
        // configured provider authenticates; everything else falls through to 401.
        let authenticated = match bearer_id {
            Some(id) => AuthManager
                .login_using_id_verified(id, false)
                .unwrap_or(false),
            None => false,
        };

        if authenticated {
            next.run(req).await
        } else {
            AppError::Unauthorized.into_response()
        }
    })
    .await
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

    /// `require_auth` must reject a request that carries no bearer token with a 401
    /// WITHOUT ever invoking the downstream handler — proving it short-circuits
    /// before any body extraction. The handler panics if reached.
    #[tokio::test]
    async fn test_require_auth_rejects_missing_token_before_handler() {
        use axum::{body::Body, http::Request as HttpRequest, routing::post, Router};
        use tower::ServiceExt;

        async fn guarded() -> &'static str {
            panic!("handler must not run for an unauthenticated request");
        }

        let app = Router::new().route(
            "/protected",
            post(guarded).route_layer(axum::middleware::from_fn(require_auth)),
        );

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/protected")
                    // A body that WOULD fail validation — but auth must win, so the
                    // body is never read and we get 401, not 422/400.
                    .header("content-type", "application/json")
                    .body(Body::from("this is not valid json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    /// A malformed/non-id bearer token is rejected with 401 too (verifying path:
    /// only a real user id authenticates).
    #[tokio::test]
    async fn test_require_auth_rejects_non_id_bearer() {
        use axum::{body::Body, http::Request as HttpRequest, routing::get, Router};
        use tower::ServiceExt;

        async fn guarded() -> &'static str {
            panic!("handler must not run for an unauthenticated request");
        }

        let app = Router::new().route(
            "/protected",
            get(guarded).route_layer(axum::middleware::from_fn(require_auth)),
        );

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer not-a-number")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
