//! Real hot-path benchmarks for RustForge
//!
//! These are NOT simulated with sleep — every benchmark exercises real code
//! paths against real data structures or real in-memory storage.
//!
//! Four paths measured:
//!   (a) AsyncBridge sync-over-async handoff overhead vs raw tokio block_on
//!   (b) HTTP handler round-trip through an axum Router (in-process oneshot)
//!   (c) SQLite INSERT and SELECT via sqlx (in-memory, single connection)
//!   (d) DTO validation via rf-validation Validator (3 fields, passing + failing)
//!
//! Run with:
//!   cargo bench -p rustforge-benchmarks --bench hot_paths
//!
//! See docs/PERFORMANCE.md for the actual numbers from the reference machine.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::collections::HashMap;

// ============================================================================
// (a) AsyncBridge overhead — bridge channel round-trip vs direct tokio block_on
// ============================================================================

fn bench_async_bridge(c: &mut Criterion) {
    // The bridge owns ONE dedicated OS thread with ONE Tokio current-thread
    // runtime. Clones share that thread; no new thread/runtime is spawned per
    // call (confirmed in crates/rf-async-bridge/src/lib.rs:90-117).
    let bridge = rf_async_bridge::AsyncBridge::new();

    // Baseline: a direct current-thread runtime block_on (no channel overhead)
    let rt_direct = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("async_bridge");
    group.throughput(Throughput::Elements(1));

    // Bridge path: caller → unbounded channel → worker tokio::spawn → sync_channel reply
    group.bench_function("bridge_block_on_noop", |b| {
        b.iter(|| bridge.block_on(async { black_box(42u64) }));
    });

    // Baseline: same-thread tokio block_on (no cross-thread channel hop)
    group.bench_function("tokio_direct_block_on_noop", |b| {
        b.iter(|| rt_direct.block_on(async { black_box(42u64) }));
    });

    group.finish();
}

// ============================================================================
// (b) HTTP handler round-trip through axum Router (tower oneshot, no network)
// ============================================================================

fn bench_http_handler(c: &mut Criterion) {
    use axum::{routing::get, Json, Router};
    use serde_json::{json, Value};
    use tower::ServiceExt; // for oneshot

    async fn json_handler() -> Json<Value> {
        Json(json!({"status": "ok", "id": 1, "name": "bench-user"}))
    }

    // Build router once; clone is cheap (Arc inside)
    // axum 0.8 uses `{param}` capture syntax (`:param` was removed in 0.8)
    let app: Router = Router::new().route("/api/users/{id}", get(json_handler));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("http");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_json_handler_oneshot", |b| {
        b.to_async(&rt).iter(|| {
            let app = app.clone();
            async move {
                let request = axum::http::Request::builder()
                    .uri("/api/users/1")
                    .header("accept", "application/json")
                    .body(axum::body::Body::empty())
                    .unwrap();
                let response = app.oneshot(request).await.unwrap();
                black_box(response.status())
            }
        });
    });

    group.finish();
}

// ============================================================================
// (c) SQLite via sqlx — in-memory INSERT and SELECT (single pool connection)
// ============================================================================

fn bench_sqlite(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    // Single connection → single in-memory database (pool size = 1 ensures
    // all queries target the same :memory: instance so the seeded row is visible)
    let pool = rt.block_on(async {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE bench_users (
                id    INTEGER PRIMARY KEY AUTOINCREMENT,
                name  TEXT NOT NULL,
                email TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Seed one row so the SELECT bench always finds something
        sqlx::query("INSERT INTO bench_users (name, email) VALUES ('seed', 'seed@example.com')")
            .execute(&pool)
            .await
            .unwrap();

        pool
    });

    let mut group = c.benchmark_group("sqlite");
    group.throughput(Throughput::Elements(1));

    // Measure write (INSERT) latency
    group.bench_function("insert", |b| {
        b.to_async(&rt).iter(|| {
            let pool = pool.clone();
            async move {
                black_box(
                    sqlx::query(
                        "INSERT INTO bench_users (name, email) VALUES ('bench', 'bench@example.com')",
                    )
                    .execute(&pool)
                    .await
                    .unwrap(),
                )
            }
        });
    });

    // Measure read (SELECT by PK) latency
    group.bench_function("select_by_id", |b| {
        b.to_async(&rt).iter(|| {
            let pool = pool.clone();
            async move {
                black_box(
                    sqlx::query("SELECT id, name, email FROM bench_users WHERE id = 1")
                        .fetch_optional(&pool)
                        .await
                        .unwrap(),
                )
            }
        });
    });

    group.finish();
}

// ============================================================================
// (d) DTO validation — rf-validation Validator, 3 fields, real rules
// ============================================================================

fn bench_validation(c: &mut Criterion) {
    use rf_validation::{
        rules::{EmailRule, MinLengthRule, RequiredRule},
        validator::{Rule, Validator},
    };

    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("validation");
    group.throughput(Throughput::Elements(1));

    // Passing DTO — all fields valid
    group.bench_function("dto_3_fields_passing", |b| {
        b.to_async(&rt).iter(|| async {
            let data: HashMap<String, serde_json::Value> = HashMap::from([
                (
                    "name".to_string(),
                    serde_json::json!("Alice Smith"),
                ),
                (
                    "email".to_string(),
                    serde_json::json!("alice@example.com"),
                ),
                (
                    "password".to_string(),
                    serde_json::json!("securepassword123"),
                ),
            ]);

            let mut v = Validator::new(data);
            v.rules(HashMap::from([
                (
                    "name",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(MinLengthRule::new(2)),
                    ],
                ),
                (
                    "email",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(EmailRule),
                    ],
                ),
                (
                    "password",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(MinLengthRule::new(8)),
                    ],
                ),
            ]));

            black_box(v.validate().await)
        });
    });

    // Failing DTO — first field fails `required`, emails and password too short
    group.bench_function("dto_3_fields_failing", |b| {
        b.to_async(&rt).iter(|| async {
            let data: HashMap<String, serde_json::Value> = HashMap::from([
                ("name".to_string(), serde_json::json!("")), // fails required
                (
                    "email".to_string(),
                    serde_json::json!("not-an-email"),
                ),
                (
                    "password".to_string(),
                    serde_json::json!("short"),
                ),
            ]);

            let mut v = Validator::new(data);
            v.rules(HashMap::from([
                (
                    "name",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(MinLengthRule::new(2)),
                    ],
                ),
                (
                    "email",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(EmailRule),
                    ],
                ),
                (
                    "password",
                    vec![
                        Box::new(RequiredRule) as Box<dyn Rule>,
                        Box::new(MinLengthRule::new(8)),
                    ],
                ),
            ]));

            black_box(v.validate().await)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_async_bridge,
    bench_http_handler,
    bench_sqlite,
    bench_validation,
);
criterion_main!(benches);
