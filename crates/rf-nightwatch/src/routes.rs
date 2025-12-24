//! Nightwatch HTTP routes

use crate::check::{CheckRegistry, CheckStatus};
use crate::dashboard::Dashboard;
use crate::metrics::MetricsRegistry;
use crate::monitor::Monitor;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde_json::json;

/// Create the Nightwatch router
pub fn nightwatch_routes() -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(readiness_handler))
        .route("/live", get(liveness_handler))
        .route("/metrics", get(metrics_handler))
        .route("/events", get(events_handler))
        .route("/dashboard", get(dashboard_handler))
        .route("/checks", get(checks_handler))
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    let results = CheckRegistry::global().run_all().await;

    let all_pass = results.iter().all(|(_, r)| r.status == CheckStatus::Pass);
    let status = if all_pass {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = json!({
        "status": if all_pass { "healthy" } else { "unhealthy" },
        "checks": results.iter().map(|(name, result)| {
            json!({
                "name": name,
                "status": format!("{:?}", result.status).to_lowercase(),
                "message": result.message,
                "duration_ms": result.duration_ms
            })
        }).collect::<Vec<_>>()
    });

    (status, Json(response))
}

/// Readiness check endpoint
async fn readiness_handler() -> impl IntoResponse {
    let results = CheckRegistry::global().run_all().await;
    let all_pass = results.iter().all(|(_, r)| r.status == CheckStatus::Pass);

    if all_pass {
        (StatusCode::OK, Json(json!({ "ready": true })))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "ready": false })),
        )
    }
}

/// Liveness check endpoint
async fn liveness_handler() -> impl IntoResponse {
    Json(json!({ "alive": true }))
}

/// Metrics endpoint
async fn metrics_handler() -> impl IntoResponse {
    let snapshot = MetricsRegistry::global().snapshot();
    Json(snapshot)
}

/// Events endpoint
async fn events_handler() -> impl IntoResponse {
    let events = Monitor::global().recent(100);
    Json(json!({
        "events": events,
        "total": Monitor::global().count()
    }))
}

/// Dashboard endpoint
async fn dashboard_handler() -> impl IntoResponse {
    let dashboard = Dashboard::generate().await;
    Json(dashboard)
}

/// Checks endpoint
async fn checks_handler() -> impl IntoResponse {
    let checks = CheckRegistry::global().list();
    Json(json!({
        "checks": checks.iter().map(|c| {
            json!({
                "name": c.name,
                "description": c.description
            })
        }).collect::<Vec<_>>()
    }))
}
