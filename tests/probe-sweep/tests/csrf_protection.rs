// Integration probe: csrf_protection
//
// Verifies the rf-web CSRF middleware (CsrfLayer / CsrfTokenStore synchronizer-token
// pattern):
//
//   1. A POST to a CSRF-protected endpoint WITHOUT a valid token is rejected with 403.
//   2. A POST WITH a valid token (registered server-side and sent in X-CSRF-TOKEN
//      header) is accepted (200).
//   3. A token is single-use: replaying the same token on a second POST is rejected.
//   4. GET requests are always allowed (CSRF only applies to state-changing methods).

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    routing::{get, post},
    Router,
};
use rf_web::{CsrfLayer, CsrfToken, CsrfTokenStore};
use tower::ServiceExt; // oneshot

/// Build a router that:
///   GET /token  — generates + registers a CSRF token, returns it as plain text
///   POST /submit — protected by the CsrfLayer; succeeds only with a valid token
///
/// The token store is shared between the handler and the layer via `Arc<RwLock<_>>`
/// inside `CsrfTokenStore`, so all clones of the store observe each other's mutations.
fn build_app() -> (Router, CsrfTokenStore) {
    let layer = CsrfLayer::new();
    let shared_store = layer.token_store().clone();

    let handler_store = shared_store.clone();
    let app = Router::new()
        .route(
            "/token",
            get(move || {
                let s = handler_store.clone();
                async move {
                    let token = CsrfToken::generate();
                    s.register(&token).await;
                    token.token().to_string()
                }
            }),
        )
        .route("/submit", post(|| async { "protected resource" }))
        .layer(layer);

    (app, shared_store)
}

/// Issue a GET /token and return the token string.
async fn fetch_token(app: Router) -> String {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "GET /token must succeed");
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Issue a POST /submit optionally carrying a CSRF token header.
async fn do_post(app: Router, csrf_token: Option<&str>) -> StatusCode {
    let mut builder = Request::builder().method(Method::POST).uri("/submit");
    if let Some(t) = csrf_token {
        builder = builder.header("X-CSRF-TOKEN", t);
    }
    let req = builder.body(Body::empty()).unwrap();
    app.oneshot(req).await.unwrap().status()
}

// ---------------------------------------------------------------------------
// 1. POST without a CSRF token → 403 Forbidden
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_post_without_csrf_token_is_403() {
    let (app, _store) = build_app();
    let status = do_post(app, None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "POST without CSRF token must be rejected with 403"
    );
}

// ---------------------------------------------------------------------------
// 2. POST with a valid CSRF token → 200 OK
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_post_with_valid_csrf_token_is_200() {
    let (app, _store) = build_app();

    // Obtain a registered token via the GET endpoint.
    let token = fetch_token(app.clone()).await;

    // POST with the valid token in the X-CSRF-TOKEN header.
    let status = do_post(app, Some(&token)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "POST with a valid CSRF token must succeed with 200"
    );
}

// ---------------------------------------------------------------------------
// 3. A CSRF token is single-use: replaying it is rejected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_csrf_token_is_single_use() {
    let (app, _store) = build_app();
    let token = fetch_token(app.clone()).await;

    // First use: must succeed.
    let first = do_post(app.clone(), Some(&token)).await;
    assert_eq!(first, StatusCode::OK, "first POST must succeed");

    // Second use with the same token: must be rejected.
    let second = do_post(app.clone(), Some(&token)).await;
    assert_eq!(
        second,
        StatusCode::FORBIDDEN,
        "replaying the same CSRF token must be rejected (single-use)"
    );
}

// ---------------------------------------------------------------------------
// 4. GET requests bypass CSRF protection entirely
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_bypasses_csrf() {
    let (app, _store) = build_app();

    // The GET /token route is protected only by CSRF on state-changing methods.
    // A GET with no token must still return 200.
    let req = Request::builder()
        .method(Method::GET)
        .uri("/token")
        .body(Body::empty())
        .unwrap();
    let status = app.oneshot(req).await.unwrap().status();
    assert_eq!(
        status,
        StatusCode::OK,
        "GET requests must not be blocked by CSRF middleware"
    );
}

// ---------------------------------------------------------------------------
// 5. An arbitrary / fabricated token is rejected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_fabricated_csrf_token_is_rejected() {
    let (app, _store) = build_app();

    let status = do_post(app, Some("attacker_fabricated_token")).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a fabricated CSRF token must be rejected"
    );
}
