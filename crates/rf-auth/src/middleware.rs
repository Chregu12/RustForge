//! Authentication middleware for Axum
//!
//! Provides middleware for protecting routes with JWT authentication.

use crate::auth_manager::{with_auth_scope, AuthManager};
use crate::jwt::JwtManager;
use crate::Claims;
use axum::{
    extract::{Extension, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
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

// ── Shared JWT validation + scope setup ───────────────────────────────────────

/// Core JWT validation + auth-scope setup shared by [`require_auth`] and
/// [`require_auth_with`].
///
/// Reads the bearer token, validates it against `jwt`, inserts [`Claims`] into
/// extensions (so `Extension<Claims>` extractors keep working), then runs the
/// handler inside a fresh [`with_auth_scope`] with the user logged in so
/// [`Auth`](crate::Auth) APIs (`check()` / `user()` / `id()`) resolve correctly.
async fn jwt_require_auth_inner(jwt: Arc<JwtManager>, mut req: Request, next: Next) -> Response {
    // Extract bearer token from header only — never touch the body, so a 401
    // always precedes any body-extractor 422 further down the stack.
    let token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned());

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return AppError::Unauthorized.into_response(),
    };

    // Validate JWT signature + expiry. Any failure → 401.
    let claims = match jwt.validate_token(&token) {
        Ok(c) => c,
        Err(_) => return AppError::Unauthorized.into_response(),
    };

    // Insert validated Claims into request extensions so that handlers using
    // `Extension<Claims>` continue to work unchanged after migration.
    req.extensions_mut().insert(claims.clone());

    // Wrap in a fresh per-request auth scope so `Auth::user()` / `Auth::check()`
    // / `Auth::id()` resolve for the authenticated user in every handler.
    with_auth_scope(async move {
        let user_json = serde_json::json!({
            "id": claims.user_id,
            "email": claims.sub,
            "roles": claims.roles,
        });
        // Login cannot fail for a well-formed JSON value; ignore the Result.
        let _ = AuthManager.login(user_json);
        next.run(req).await
    })
    .await
}

/// Ready-made JWT bearer-auth guard: rejects unauthenticated requests with a
/// JSON 401 **before** the handler or any of its body extractors run.
///
/// This middleware:
///
/// 1. Reads the `Authorization: Bearer <jwt>` header — **headers only, never the
///    body** (so auth wins over any 422 a body-validator would otherwise raise);
/// 2. Validates the token via [`JwtManager`] read from an Axum [`Extension`] —
///    register it once at router build time with `.layer(Extension(jwt.clone()))`;
/// 3. On success, opens a fresh per-request auth scope (like [`auth_scope`]) so
///    [`Auth`](crate::Auth) reads are isolated across concurrent requests, and
///    logs the authenticated user (id / email / roles from JWT claims) into that
///    scope so `Auth::user()`, `Auth::check()`, and `Auth::id()` all work in
///    downstream handlers;
/// 4. Also inserts the decoded [`Claims`] into request extensions so handlers
///    that extract `Extension<Claims>` keep working without changes;
/// 5. On a missing, malformed, expired, or tampered token — or when no
///    [`JwtManager`] extension is configured (fail-closed) — short-circuits with
///    an [`AppError::Unauthorized`] 401 JSON envelope before any handler body
///    code executes.
///
/// # Quick start
///
/// ```ignore
/// use axum::{Router, routing::get, middleware, Extension};
/// use rf_auth::{require_auth, JwtManager};
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("your-secret-key-min-32-characters")?);
///
/// let app = Router::new()
///     .route("/profile", get(profile_handler))
///     .route_layer(middleware::from_fn(require_auth))
///     .layer(Extension(jwt));
/// ```
///
/// If you cannot easily add an [`Extension`] layer (e.g. the [`JwtManager`]
/// lives inside your app state struct), use [`require_auth_with`] instead.
pub async fn require_auth(req: Request, next: Next) -> Response {
    // Read the JwtManager from extensions; fail-closed if it was not configured.
    let jwt = req.extensions().get::<Arc<JwtManager>>().cloned();
    match jwt {
        Some(jwt) => jwt_require_auth_inner(jwt, req, next).await,
        None => AppError::Unauthorized.into_response(),
    }
}

/// Alternative to [`require_auth`] for cases where the [`JwtManager`] lives
/// inside your application state and cannot easily be provided via
/// [`Extension`].
///
/// Call this at router-build time with the pre-constructed manager to get a
/// closure compatible with `axum::middleware::from_fn`:
///
/// ```ignore
/// use axum::{Router, routing::post, middleware};
/// use rf_auth::{require_auth_with, JwtManager};
/// use std::sync::Arc;
///
/// let jwt = Arc::new(JwtManager::new("your-secret-key-min-32-characters")?);
///
/// let protected = Router::new()
///     .route("/posts", post(create_post))
///     .route_layer(middleware::from_fn(require_auth_with(jwt)));
/// ```
///
/// The returned closure is `Clone + Send + 'static` so it satisfies Axum's
/// `from_fn` constraints. It applies exactly the same JWT validation, auth-scope
/// setup, and `Extension<Claims>` injection as [`require_auth`].
pub fn require_auth_with(
    manager: Arc<JwtManager>,
) -> impl Fn(Request, Next) -> BoxFuture<'static, Response> + Clone + Send + 'static {
    move |req: Request, next: Next| {
        let manager = manager.clone();
        Box::pin(async move { jwt_require_auth_inner(manager, req, next).await })
    }
}

/// Authentication middleware that validates JWT tokens
///
/// Extracts the JWT token from the Authorization header,
/// validates it, and adds the claims to request extensions.
///
/// **Prefer [`require_auth`] or [`require_auth_with`]** for new code — they
/// additionally set up the per-request [`AuthManager`] scope so
/// `Auth::user()` / `Auth::check()` / `Auth::id()` work in handlers.
/// `auth_middleware` is kept for backward compatibility.
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
/// **Prefer [`require_auth`]** for new code — it additionally sets up the
/// per-request [`AuthManager`] scope so `Auth::user()` / `Auth::check()` /
/// `Auth::id()` work in handlers. `auth_layer` is kept for backward
/// compatibility.
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
    use crate::{Claims, JwtManager};

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

    /// `require_auth` must reject a request that carries no bearer token with a
    /// 401 WITHOUT ever invoking the downstream handler — proving it
    /// short-circuits before any body extraction. The handler panics if reached.
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

    /// A malformed bearer token is rejected with 401 — no `JwtManager` in
    /// extensions means no token (however well or badly formed) can pass.
    /// Fail-closed: unconfigured guard never admits requests.
    #[tokio::test]
    async fn test_require_auth_rejects_invalid_bearer_without_manager() {
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
                    .header("Authorization", "Bearer not-a-jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    // ── JWT-capability integration tests ─────────────────────────────────────

    /// Helper: build a minimal Axum app with `require_auth` guarding one route.
    /// The JwtManager is provided via Extension so `require_auth` can validate.
    fn jwt_guarded_app(jwt: Arc<JwtManager>) -> axum::Router {
        use axum::{routing::get, Extension, Router};

        async fn protected(Extension(claims): Extension<Claims>) -> String {
            format!("user_id={}", claims.user_id)
        }

        Router::new()
            .route("/protected", get(protected))
            .route_layer(axum::middleware::from_fn(require_auth))
            .layer(Extension(jwt))
    }

    /// A request carrying a VALID JWT:
    ///  - returns 200 with the handler's response,
    ///  - populates `Extension<Claims>` so the handler reads the user id,
    ///  - sets up the AuthManager scope so `Auth::id()` resolves inside the
    ///    handler (verified via the formatted response body).
    #[tokio::test]
    async fn test_require_auth_valid_jwt_passes_200_and_populates_auth() {
        use axum::{body::Body, http::Request as HttpRequest};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());
        let claims = Claims::new(42, "alice@example.com".into(), vec!["user".into()], 24);
        let token = jwt.generate_token(&claims).unwrap();

        let app = jwt_guarded_app(jwt);

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert_eq!(body, "user_id=42", "handler must see the jwt user_id=42");
    }

    /// A request with NO Authorization header → 401, even with a JwtManager present.
    #[tokio::test]
    async fn test_require_auth_missing_token_returns_401_with_manager() {
        use axum::{body::Body, http::Request as HttpRequest};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());
        let app = jwt_guarded_app(jwt);

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    /// An expired JWT token → 401 (the JwtManager's validate_token rejects it).
    #[tokio::test]
    async fn test_require_auth_expired_jwt_returns_401() {
        use axum::{body::Body, http::Request as HttpRequest};
        use chrono::{Duration, Utc};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());

        // Forge an already-expired claims by backdating exp
        let mut claims = Claims::new(1, "bob@example.com".into(), vec![], 1);
        claims.exp = (Utc::now() - Duration::hours(2)).timestamp();
        let token = jwt.generate_token(&claims).unwrap();

        let app = jwt_guarded_app(jwt);

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    /// A JWT signed with a different secret key → 401 (tampered / wrong issuer).
    #[tokio::test]
    async fn test_require_auth_tampered_jwt_returns_401() {
        use axum::{body::Body, http::Request as HttpRequest};
        use tower::ServiceExt;

        let real_jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());
        let attacker_jwt =
            JwtManager::new("attacker-different-secret-key-32ch!!").unwrap();

        // Token signed by attacker with a different key
        let claims = Claims::new(99, "attacker@evil.com".into(), vec!["admin".into()], 24);
        let token = attacker_jwt.generate_token(&claims).unwrap();

        let app = jwt_guarded_app(real_jwt);

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    /// A completely malformed (non-JWT) bearer value → 401.
    #[tokio::test]
    async fn test_require_auth_malformed_bearer_returns_401() {
        use axum::{body::Body, http::Request as HttpRequest};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());
        let app = jwt_guarded_app(jwt);

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer not.a.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    // ── require_auth_with tests ───────────────────────────────────────────────

    /// `require_auth_with` is identical to `require_auth` except the caller
    /// supplies the manager directly instead of via Extension.
    #[tokio::test]
    async fn test_require_auth_with_valid_jwt_passes() {
        use axum::{body::Body, http::Request as HttpRequest, routing::get, Router};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());
        let claims = Claims::new(7, "carol@example.com".into(), vec!["user".into()], 1);
        let token = jwt.generate_token(&claims).unwrap();

        async fn handler(Extension(c): Extension<Claims>) -> String {
            format!("id={}", c.user_id)
        }

        let app = Router::new()
            .route("/", get(handler))
            .route_layer(axum::middleware::from_fn(require_auth_with(jwt)));

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(String::from_utf8(bytes.to_vec()).unwrap(), "id=7");
    }

    /// `require_auth_with` rejects a missing token with 401.
    #[tokio::test]
    async fn test_require_auth_with_missing_token_returns_401() {
        use axum::{body::Body, http::Request as HttpRequest, routing::get, Router};
        use tower::ServiceExt;

        let jwt = Arc::new(JwtManager::new(TEST_SECRET).unwrap());

        async fn handler() -> &'static str {
            panic!("must not run");
        }

        let app = Router::new()
            .route("/", get(handler))
            .route_layer(axum::middleware::from_fn(require_auth_with(jwt)));

        let resp = app
            .oneshot(
                HttpRequest::builder()
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
    }
}
