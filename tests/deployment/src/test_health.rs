//! Deployment tests for rf-health

#[cfg(test)]
mod tests {
    use rf_health::{
        HealthChecker, HealthCheck, HealthStatus, HealthResponse, CheckResult,
    };
    use rf_health::checks::{AlwaysHealthyCheck, MemoryCheck, DiskCheck, UptimeCheck};

    // ── HealthStatus ─────────────────────────────────────────────

    #[test]
    fn health_status_variants() {
        assert!(HealthStatus::Healthy.is_healthy());
        assert!(!HealthStatus::Healthy.is_degraded());
        assert!(!HealthStatus::Healthy.is_unhealthy());

        assert!(HealthStatus::Degraded.is_degraded());
        assert!(!HealthStatus::Degraded.is_healthy());

        assert!(HealthStatus::Unhealthy.is_unhealthy());
        assert!(!HealthStatus::Unhealthy.is_healthy());
    }

    #[test]
    fn health_status_http_codes() {
        assert_eq!(HealthStatus::Healthy.http_status(), 200);
        assert_eq!(HealthStatus::Degraded.http_status(), 200);
        assert_eq!(HealthStatus::Unhealthy.http_status(), 503);
    }

    // ── CheckResult ──────────────────────────────────────────────

    #[test]
    fn check_result_healthy() {
        let result = CheckResult::healthy("database");
        assert_eq!(result.name, "database");
        assert!(result.status.is_healthy());
        assert!(result.message.is_none());
    }

    #[test]
    fn check_result_degraded() {
        let result = CheckResult::degraded("cache", "High latency");
        assert!(result.status.is_degraded());
        assert_eq!(result.message, Some("High latency".into()));
    }

    #[test]
    fn check_result_unhealthy() {
        let result = CheckResult::unhealthy("redis", "Connection refused");
        assert!(result.status.is_unhealthy());
    }

    #[test]
    fn check_result_with_metadata() {
        let result = CheckResult::healthy("database")
            .with_metadata("version", serde_json::json!("14.2"))
            .with_metadata("connections", serde_json::json!(42));
        assert_eq!(result.metadata.len(), 2);
    }

    // ── HealthResponse ───────────────────────────────────────────

    #[test]
    fn health_response_all_healthy() {
        let response = HealthResponse::from_checks(vec![
            CheckResult::healthy("db"),
            CheckResult::healthy("cache"),
        ]);
        assert!(response.status.is_healthy());
        assert_eq!(response.http_status(), 200);
        assert_eq!(response.checks.len(), 2);
    }

    #[test]
    fn health_response_degraded() {
        let response = HealthResponse::from_checks(vec![
            CheckResult::healthy("db"),
            CheckResult::degraded("cache", "slow"),
        ]);
        assert!(response.status.is_degraded());
    }

    #[test]
    fn health_response_unhealthy() {
        let response = HealthResponse::from_checks(vec![
            CheckResult::healthy("db"),
            CheckResult::unhealthy("redis", "down"),
        ]);
        assert!(response.status.is_unhealthy());
        assert_eq!(response.http_status(), 503);
    }

    // ── Built-in Health Checks ───────────────────────────────────

    #[tokio::test]
    async fn always_healthy_check() {
        let check = AlwaysHealthyCheck::new("test");
        assert_eq!(check.name(), "test");
        let result = check.check().await;
        assert!(result.status.is_healthy());
    }

    #[tokio::test]
    async fn memory_check() {
        let check = MemoryCheck::default();
        let result = check.check().await;
        // On any running system, memory check should succeed
        assert!(result.status.is_healthy() || result.status.is_degraded());
    }

    #[tokio::test]
    async fn disk_check() {
        let check = DiskCheck::default();
        let result = check.check().await;
        // Disk check returns healthy, degraded, or unhealthy depending on environment
        let _ = result.status; // just verify it completes without panic
    }

    #[tokio::test]
    async fn uptime_check() {
        let check = UptimeCheck::new();
        let result = check.check().await;
        assert!(result.status.is_healthy());
    }

    // ── HealthChecker ────────────────────────────────────────────

    #[tokio::test]
    async fn health_checker_all() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("service_a"))
            .add_check(AlwaysHealthyCheck::new("service_b"))
            .add_check(UptimeCheck::new());

        let response = checker.check_all().await;
        assert!(response.status.is_healthy());
        assert_eq!(response.checks.len(), 3);
    }

    #[tokio::test]
    async fn health_checker_liveness() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("basic"));

        let response = checker.check_liveness().await;
        assert!(response.status.is_healthy());
    }

    #[tokio::test]
    async fn health_checker_readiness() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("ready_check"))
            .add_check(MemoryCheck::default());

        let response = checker.check_readiness().await;
        assert!(response.status.is_healthy() || response.status.is_degraded());
    }

    // ── Health Router ────────────────────────────────────────────

    #[test]
    fn health_router_builds() {
        let checker = HealthChecker::new()
            .add_check(AlwaysHealthyCheck::new("test"));
        let router = rf_health::health_router(checker);
        let _ = router; // verify it compiles and builds
    }
}
