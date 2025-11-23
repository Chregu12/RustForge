mod common;

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn invoke_returns_success_for_list_command() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let request_body = serde_json::json!({
        "command": "list",
        "format": "json"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("serialize body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json payload");

    assert_eq!(payload["status"], "success");
    assert!(
        payload["data"].get("commands").is_some(),
        "list command should include command catalog"
    );
}

#[tokio::test]
async fn invoke_returns_not_found_for_unknown_command() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let request_body = serde_json::json!({
        "command": "does-not-exist"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("serialize body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json payload");
    assert_eq!(payload["code"], "COMMAND_NOT_FOUND");
    assert_eq!(payload["status"], 404);
    assert_eq!(
        payload["message"],
        "Command `does-not-exist` wurde nicht gefunden"
    );
}

#[tokio::test]
async fn invoke_validates_presence_of_command() {
    let harness = common::HttpTestApp::new();
    let router = harness.router();

    let request = Request::builder()
        .method(Method::POST)
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("serialize body");
    let payload: Value = serde_json::from_slice(&bytes).expect("json payload");
    assert_eq!(payload["code"], "VALIDATION");
    assert_eq!(payload["status"], 422);
    let errors = payload["errors"].as_array().expect("validation errors");
    assert!(errors.iter().any(|entry| {
        entry["field"].as_str() == Some("command") && entry["message"].as_str().is_some()
    }));
}
