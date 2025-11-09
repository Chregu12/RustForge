//! Axum endpoint integration

use crate::checker::{HealthCheck, HealthResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::sync::Arc;

/// Health checker that runs multiple checks
#[derive(Clone)]
pub struct HealthChecker {
    checks: Arc<Vec<Arc<dyn HealthCheck>>>,
    liveness_checks: Arc<Vec<Arc<dyn HealthCheck>>>,
    readiness_checks: Arc<Vec<Arc<dyn HealthCheck>>>,
}

impl HealthChecker {
    /// Create new health checker
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Vec::new()),
            liveness_checks: Arc::new(Vec::new()),
            readiness_checks: Arc::new(Vec::new()),
        }
    }

    /// Add a health check
    pub fn add_check(mut self, check: impl HealthCheck + 'static) -> Self {
        let check = Arc::new(check);
        let checks = Arc::make_mut(&mut self.checks);

        // Also add to liveness or readiness lists
        if check.is_liveness() {
            let liveness_checks = Arc::make_mut(&mut self.liveness_checks);
            liveness_checks.push(Arc::clone(&check) as Arc<dyn HealthCheck>);
        }

        if check.is_readiness() {
            let readiness_checks = Arc::make_mut(&mut self.readiness_checks);
            readiness_checks.push(Arc::clone(&check) as Arc<dyn HealthCheck>);
        }

        checks.push(check as Arc<dyn HealthCheck>);
        self
    }

    /// Run all health checks
    pub async fn check_all(&self) -> HealthResponse {
        let mut results = Vec::new();

        for check in self.checks.iter() {
            let result = check.check().await;
            results.push(result);
        }

        HealthResponse::from_checks(results)
    }

    /// Run liveness checks only
    pub async fn check_liveness(&self) -> HealthResponse {
        let mut results = Vec::new();

        for check in self.liveness_checks.iter() {
            let result = check.check().await;
            results.push(result);
        }

        // If no liveness checks, return healthy
        if results.is_empty() {
            return HealthResponse::from_checks(vec![]);
        }

        HealthResponse::from_checks(results)
    }

    /// Run readiness checks only
    pub async fn check_readiness(&self) -> HealthResponse {
        let mut results = Vec::new();

        for check in self.readiness_checks.iter() {
            let result = check.check().await;
            results.push(result);
        }

        // If no readiness checks, return all checks
        if results.is_empty() {
            return self.check_all().await;
        }

        HealthResponse::from_checks(results)
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Health endpoint handler
async fn health_handler(State(checker): State<HealthChecker>) -> Response {
    let response = checker.check_all().await;
    let status = StatusCode::from_u16(response.http_status()).unwrap_or(StatusCode::OK);

    (status, Json(response)).into_response()
}

/// Liveness endpoint handler (Kubernetes)
async fn liveness_handler(State(checker): State<HealthChecker>) -> Response {
    let response = checker.check_liveness().await;
    let status = StatusCode::from_u16(response.http_status()).unwrap_or(StatusCode::OK);

    (status, Json(response)).into_response()
}

/// Readiness endpoint handler (Kubernetes)
async fn readiness_handler(State(checker): State<HealthChecker>) -> Response {
    let response = checker.check_readiness().await;
    let status = StatusCode::from_u16(response.http_status()).unwrap_or(StatusCode::OK);

    (status, Json(response)).into_response()
}

/// Create health check router
///
/// # Example
///
/// ```no_run
/// use rf_health::{health_router, HealthChecker};
/// use rf_health::checks::{MemoryCheck, DiskCheck};
/// use axum::Router;
///
/// # async fn example() {
/// let checker = HealthChecker::new()
///     .add_check(MemoryCheck::default())
///     .add_check(DiskCheck::default());
///
/// let app = Router::new()
///     .merge(health_router(checker));
/// # }
/// ```
pub fn health_router(checker: HealthChecker) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/health/live", get(liveness_handler))
        .route("/health/ready", get(readiness_handler))
        .with_state(checker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::AlwaysHealthyCheck;

    #[tokio::test]
    async fn test_health_checker() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("test1"))
            .add_check(AlwaysHealthyCheck::new("test2"));

        let response = checker.check_all().await;

        assert_eq!(response.checks.len(), 2);
        assert!(response.status.is_healthy());
    }

    #[tokio::test]
    async fn test_empty_liveness() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("test"));

        let response = checker.check_liveness().await;

        // No liveness checks, should return empty (healthy)
        assert_eq!(response.checks.len(), 0);
        assert!(response.status.is_healthy());
    }
}
