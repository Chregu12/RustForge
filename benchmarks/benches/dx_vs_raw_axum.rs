//! RustForge DX layer vs raw axum — reproducible overhead benchmark
//!
//! Measures the per-request cost of the `capture_request` middleware +
//! task-local scope + `input()` helper (the "DX layer") relative to an
//! equivalent plain axum handler using typed extractors.
//!
//! Two endpoint shapes are compared:
//!
//!   1. **GET /users/{id}** — no request body; a path-parameter read.
//!      - Raw axum: `Path(id): Path<i64>` typed extractor.
//!      - RustForge DX: `capture_request` outer layer + `capture_path_params`
//!        route_layer; handler has **no arguments** and calls `input("id")`.
//!
//!   2. **POST /echo** — small JSON body (`{"title":"hello"}`); a body-field read.
//!      - Raw axum: `Json(body): Json<Value>` extractor; reads `body["title"]`.
//!      - RustForge DX: `capture_request` outer layer; handler has **no
//!        arguments** and calls `input::<String>("title")`.
//!
//! All requests go through `tower::ServiceExt::oneshot` — no TCP stack,
//! no network I/O — so numbers reflect purely the in-process middleware
//! and routing overhead.
//!
//! ## Reproduction
//!
//! ```bash
//! cargo bench -p rustforge-benchmarks --bench dx_vs_raw_axum
//! # Quick pass (shorter sampling):
//! cargo bench -p rustforge-benchmarks --bench dx_vs_raw_axum -- \
//!     --measurement-time 5 --warm-up-time 2
//! ```
//!
//! See `docs/PERFORMANCE.md` §"RustForge DX vs raw axum" for recorded numbers.

use axum::{
    extract::Path,
    middleware,
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use rf_request::{capture_path_params, capture_request, input};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Shared response type
// ============================================================================

#[derive(serde::Serialize)]
struct UserResponse {
    id: i64,
    name: &'static str,
    email: &'static str,
}

// ============================================================================
// GET /users/{id} — path-parameter read
// ============================================================================

/// Raw axum handler: typed `Path` extractor, zero framework overhead.
async fn raw_get_user(Path(id): Path<i64>) -> Json<UserResponse> {
    Json(UserResponse {
        id: black_box(id),
        name: "bench-user",
        email: "bench@example.com",
    })
}

/// RustForge DX handler: argument-less; reads path param via `input()`.
/// Requires `capture_request` outer layer + `capture_path_params` route_layer.
async fn dx_get_user() -> Json<UserResponse> {
    let id: i64 = input("id").unwrap_or(0);
    Json(UserResponse {
        id: black_box(id),
        name: "bench-user",
        email: "bench@example.com",
    })
}

// ============================================================================
// POST /echo — JSON body field read
// ============================================================================

/// Raw axum handler: `Json` extractor reads the body; returns the title field.
async fn raw_post_echo(Json(body): Json<Value>) -> Json<Value> {
    let title = body["title"].as_str().unwrap_or("").to_string();
    Json(json!({ "title": black_box(title) }))
}

/// RustForge DX handler: argument-less; reads body field via `input()`.
/// Requires `capture_request` outer layer (buffers + parses the JSON body).
async fn dx_post_echo() -> Json<Value> {
    let title: String = input("title").unwrap_or_default();
    Json(json!({ "title": black_box(title) }))
}

// ============================================================================
// Benchmark: GET path-param
// ============================================================================

fn bench_get_path_param(c: &mut Criterion) {
    // --- Raw axum router (no extra middleware) ---
    let raw_app: Router = Router::new().route("/users/{id}", get(raw_get_user));

    // --- RustForge DX router ---
    // capture_path_params is a route_layer (runs after route matching so
    // axum has already populated RawPathParams).
    // capture_request is an outer layer (runs before the handler, buffers body).
    let dx_app: Router = Router::new()
        .route(
            "/users/{id}",
            get(dx_get_user).route_layer(middleware::from_fn(capture_path_params)),
        )
        .layer(middleware::from_fn(capture_request));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("dx_vs_raw/get_path_param");
    group.throughput(Throughput::Elements(1));

    group.bench_function("raw_axum", |b| {
        b.to_async(&rt).iter(|| {
            let app = raw_app.clone();
            async move {
                let req = axum::http::Request::builder()
                    .method("GET")
                    .uri("/users/42")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.bench_function("rustforge_dx", |b| {
        b.to_async(&rt).iter(|| {
            let app = dx_app.clone();
            async move {
                let req = axum::http::Request::builder()
                    .method("GET")
                    .uri("/users/42")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: POST JSON body field
// ============================================================================

fn bench_post_json_body(c: &mut Criterion) {
    // Pre-allocate body bytes once — Bytes is reference-counted so cloning is
    // O(1); each bench iteration wraps a fresh Body around the same arc.
    let body_bytes: Bytes = Bytes::from_static(b"{\"title\":\"hello\"}");

    // --- Raw axum router (no extra middleware) ---
    let raw_app: Router = Router::new().route("/echo", post(raw_post_echo));

    // --- RustForge DX router ---
    let dx_app: Router = Router::new()
        .route("/echo", post(dx_post_echo))
        .layer(middleware::from_fn(capture_request));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("dx_vs_raw/post_json_body");
    group.throughput(Throughput::Elements(1));

    group.bench_function("raw_axum", |b| {
        b.to_async(&rt).iter(|| {
            let app = raw_app.clone();
            let body = body_bytes.clone();
            async move {
                let req = axum::http::Request::builder()
                    .method("POST")
                    .uri("/echo")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.bench_function("rustforge_dx", |b| {
        b.to_async(&rt).iter(|| {
            let app = dx_app.clone();
            let body = body_bytes.clone();
            async move {
                let req = axum::http::Request::builder()
                    .method("POST")
                    .uri("/echo")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(body))
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: capture_request overhead in isolation (GET, no body, no path params)
// ============================================================================
//
// Isolates ONLY the middleware itself: same handler on both sides, only
// difference is whether `capture_request` wraps it. This strips out the
// routing + path-param overhead of the full comparison above.

fn bench_middleware_only(c: &mut Criterion) {
    async fn noop_handler() -> &'static str {
        "ok"
    }

    let raw_app: Router = Router::new().route("/", get(noop_handler));
    let dx_app: Router = Router::new()
        .route("/", get(noop_handler))
        .layer(middleware::from_fn(capture_request));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("dx_vs_raw/middleware_isolation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("raw_axum_no_middleware", |b| {
        b.to_async(&rt).iter(|| {
            let app = raw_app.clone();
            async move {
                let req = axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.bench_function("capture_request_empty_get", |b| {
        b.to_async(&rt).iter(|| {
            let app = dx_app.clone();
            async move {
                let req = axum::http::Request::builder()
                    .uri("/")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let resp = app.oneshot(req).await.unwrap();
                black_box(resp.status())
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_get_path_param,
    bench_post_json_body,
    bench_middleware_only,
);
criterion_main!(benches);
