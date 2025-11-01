use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::{get, post},
};
use foundry_api::{app_router, ApiResult, AppJson, AppState, JsonResponse};
use foundry_plugins::ValidationRules;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;

async fn custom_handler() -> JsonResponse<Value> {
    JsonResponse::ok(json!({"message": "hello"}))
}

async fn secure_handler() -> JsonResponse<Value> {
    JsonResponse::ok(json!({"secure": true}))
}

async fn token_guard(req: axum::http::Request<Body>, next: Next) -> impl IntoResponse {
    let authorized = req
        .headers()
        .get("x-api-token")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == "secret")
        .unwrap_or(false);

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    next.run(req).await
}

async fn echo_handler(AppJson(payload): AppJson<Value>) -> ApiResult<Value> {
    Ok(JsonResponse::ok(payload))
}

#[derive(Serialize, Deserialize)]
struct NamePayload {
    name: Option<String>,
}

async fn validation_handler(
    State(state): State<AppState>,
    payload: AppJson<NamePayload>,
) -> ApiResult<Value> {
    let rules = ValidationRules {
        rules: serde_json::json!({
            "required": ["name"],
            "fields": {
                "name": { "min_length": 3 }
            }
        }),
    };
    payload.validate(&state, rules).await?;
    let body = payload.into_inner();
    Ok(JsonResponse::ok(json!({ "name": body.name })))
}

#[tokio::test]
async fn merge_router_mounts_custom_routes() {
    let harness = common::HttpTestApp::with_server(|server| {
        let extra_routes = app_router().route("/custom", get(custom_handler));
        server.merge_router(extra_routes)
    });

    let router = harness.router();
    let request = Request::builder()
        .method(Method::GET)
        .uri("/custom")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload: Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(payload["data"]["message"], "hello");
}

#[tokio::test]
async fn middleware_blocks_unauthorized_requests() {
    let harness = common::HttpTestApp::with_server(|server| {
        let extra_routes = app_router().route("/secure", get(secure_handler));
        server
            .merge_router(extra_routes)
            .with_middleware(token_guard)
    });

    let router = harness.router();

    let unauthorized = Request::builder()
        .method(Method::GET)
        .uri("/secure")
        .body(Body::empty())
        .unwrap();
    let response = router
        .clone()
        .oneshot(unauthorized)
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let authorized = Request::builder()
        .method(Method::GET)
        .uri("/secure")
        .header("x-api-token", "secret")
        .body(Body::empty())
        .unwrap();
    let response = router.oneshot(authorized).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn app_json_maps_invalid_payloads_to_http_error() {
    let harness = common::HttpTestApp::with_server(|server| {
        let routes = app_router().route("/echo", post(echo_handler));
        server.merge_router(routes)
    });

    let router = harness.router();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/echo")
        .header("content-type", "application/json")
        .body(Body::from("{ invalid json }"))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");
    assert_eq!(payload["code"], "INVALID_JSON");
}

#[tokio::test]
async fn app_json_produces_validation_errors() {
    let harness = common::HttpTestApp::with_server(|server| {
        let routes = app_router().route("/names", post(validation_handler));
        server.merge_router(routes)
    });

    let router = harness.router();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/names")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = router.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let payload: Value = serde_json::from_slice(&bytes).expect("json body");
    assert_eq!(payload["code"], "VALIDATION");
    assert!(payload["errors"].as_array().is_some());
    let error_fields: Vec<_> = payload["errors"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["field"].as_str())
        .collect();
    assert!(error_fields.contains(&"name"));
}
