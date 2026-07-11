// Integration probe: session_per_client
// Adapted from sandbox/probes/session_per_client/src/main.rs
// Proves per-client session isolation + one-request flash over HTTP.

use axum::{
    body::Body,
    extract::Path,
    http::{header, Request},
    middleware,
    routing::{get, post},
    Router,
};
use rf::core::{session_scope, SessionFacade};
use serde_json::json;
use tower::ServiceExt; // oneshot

fn app() -> Router {
    Router::new()
        .route(
            "/put/{key}/{val}",
            post(|Path((k, v)): Path<(String, String)>| async move {
                SessionFacade::put(k, json!(v));
                "ok".to_string()
            }),
        )
        .route(
            "/flash/{key}/{val}",
            post(|Path((k, v)): Path<(String, String)>| async move {
                SessionFacade::flash(k, json!(v));
                "ok".to_string()
            }),
        )
        .route(
            "/get/{key}",
            get(|Path(k): Path<String>| async move {
                match SessionFacade::get(&k) {
                    Some(v) => v.as_str().unwrap_or("none").to_string(),
                    None => "none".to_string(),
                }
            }),
        )
        .layer(middleware::from_fn(session_scope))
}

async fn call(method: &str, uri: &str, cookie: Option<&str>) -> (String, String) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(c) = cookie {
        builder = builder.header(header::COOKIE, format!("rf_session={c}"));
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app().oneshot(req).await.unwrap();

    let set_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            s.split(';').next().and_then(|kv| {
                let mut p = kv.trim().splitn(2, '=');
                (p.next()? == "rf_session").then(|| p.next().map(|x| x.to_string()))?
            })
        })
        .expect("server must set an rf_session cookie");

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (String::from_utf8_lossy(&bytes).to_string(), set_cookie)
}

#[tokio::test]
async fn test_session_per_client() {
    // 1. Per-client isolation: A's value must not leak to B.
    let (body, cookie_a) = call("POST", "/put/secret/AAA", None).await;
    assert_eq!(body, "ok");

    let (b_read, cookie_b) = call("GET", "/get/secret", None).await;
    assert_eq!(
        b_read, "none",
        "LEAK: client B saw client A's session value"
    );
    assert_ne!(cookie_a, cookie_b, "each client must get its OWN session id");

    let (a_read, _) = call("GET", "/get/secret", Some(&cookie_a)).await;
    assert_eq!(a_read, "AAA", "client A must still see its own value");

    let _ = call("POST", "/put/secret/BBB", Some(&cookie_b)).await;
    let (a_again, _) = call("GET", "/get/secret", Some(&cookie_a)).await;
    let (b_again, _) = call("GET", "/get/secret", Some(&cookie_b)).await;
    assert_eq!(a_again, "AAA", "A/B sessions must not cross-contaminate");
    assert_eq!(b_again, "BBB", "A/B sessions must not cross-contaminate");

    // 2. One-request flash lifecycle for a single client.
    let (_, cookie_c) = call("POST", "/flash/msg/hello", None).await;
    let (r2, _) = call("GET", "/get/msg", Some(&cookie_c)).await;
    assert_eq!(r2, "hello", "flash must be readable on the NEXT request");
    let (r3, _) = call("GET", "/get/msg", Some(&cookie_c)).await;
    assert_eq!(r3, "none", "flash must be cleared after exactly one request");

    // 3. A flash for one client never leaks to another client.
    let (_, cookie_d) = call("POST", "/flash/note/for-D", None).await;
    let (e_read, _) = call("GET", "/get/note", None).await;
    assert_eq!(e_read, "none", "LEAK: another client saw D's flash");
    let (d_read, _) = call("GET", "/get/note", Some(&cookie_d)).await;
    assert_eq!(d_read, "for-D", "D must still receive its own flash once");
}
