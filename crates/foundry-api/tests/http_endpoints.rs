mod common;

use axum::{
    body::to_bytes,
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn commands_endpoint_lists_registered_commands() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let request = Request::builder()
        .method(Method::GET)
        .uri("/commands")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let payload: Value = serde_json::from_slice(&bytes).expect("parse commands payload");

    let commands = payload.as_array().expect("commands payload is array");
    assert!(
        !commands.is_empty(),
        "expected at least one registered command"
    );

    assert!(
        commands
            .iter()
            .filter_map(|entry| entry.get("name").and_then(Value::as_str))
            .any(|name| name == "list"),
        "expected catalog to include the list command"
    );
}
