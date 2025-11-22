//! Health check system for monitoring component status
//!
//! Provides health checks for various components of the application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

/// Overall health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Some non-critical issues detected
    Degraded,
    /// Critical issues detected
    Unhealthy,
}

impl HealthStatus {
    /// Get HTTP status code for this health status
    pub fn http_status(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 200,
            HealthStatus::Unhealthy => 503,
        }
    }
}

/// Component-specific health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Current status of the component
    pub status: HealthStatus,
    /// Optional message describing the status
    pub message: Option<String>,
    /// Last time this component was checked
    pub last_check: Option<SystemTime>,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
}

impl ComponentHealth {
    /// Create a healthy component status
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
            last_check: Some(SystemTime::now()),
            response_time_ms: None,
        }
    }

    /// Create a degraded component status
    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            last_check: Some(SystemTime::now()),
            response_time_ms: None,
        }
    }

    /// Create an unhealthy component status
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            last_check: Some(SystemTime::now()),
            response_time_ms: None,
        }
    }

    /// Set the response time
    pub fn with_response_time(mut self, ms: u64) -> Self {
        self.response_time_ms = Some(ms);
        self
    }
}

/// Overall health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Overall system status
    pub status: HealthStatus,
    /// Individual component statuses
    pub components: HashMap<String, ComponentHealth>,
    /// Timestamp of the health check
    pub timestamp: SystemTime,
    /// Application version
    pub version: String,
}

impl HealthCheck {
    /// Create a new health check
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            components: HashMap::new(),
            timestamp: SystemTime::now(),
            version: version.into(),
        }
    }

    /// Add a component health status
    pub fn add_component(&mut self, name: impl Into<String>, health: ComponentHealth) {
        self.components.insert(name.into(), health);
        self.update_overall_status();
    }

    /// Update the overall status based on component statuses
    fn update_overall_status(&mut self) {
        let has_unhealthy = self
            .components
            .values()
            .any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = self
            .components
            .values()
            .any(|c| c.status == HealthStatus::Degraded);

        self.status = if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }
}

/// Health checker trait for components
#[async_trait::async_trait]
pub trait HealthChecker: Send + Sync {
    /// Check the health of this component
    async fn check_health(&self) -> ComponentHealth;
}

/// Health check registry
pub struct HealthRegistry {
    checkers: Arc<RwLock<HashMap<String, Arc<dyn HealthChecker>>>>,
    version: String,
}

impl HealthRegistry {
    /// Create a new health registry
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            checkers: Arc::new(RwLock::new(HashMap::new())),
            version: version.into(),
        }
    }

    /// Register a health checker
    pub fn register(&self, name: impl Into<String>, checker: Arc<dyn HealthChecker>) {
        let mut checkers = self.checkers.write().unwrap();
        checkers.insert(name.into(), checker);
    }

    /// Run all health checks
    pub async fn check_all(&self) -> HealthCheck {
        let mut health = HealthCheck::new(&self.version);

        let checkers = {
            let guard = self.checkers.read().unwrap();
            guard.clone()
        };

        for (name, checker) in checkers {
            let component_health = checker.check_health().await;
            health.add_component(name, component_health);
        }

        health
    }

    /// Get a quick health status (cached for performance)
    pub fn quick_check(&self) -> HealthStatus {
        // For now, just return healthy
        // In production, this could use cached results
        HealthStatus::Healthy
    }
}

impl Clone for HealthRegistry {
    fn clone(&self) -> Self {
        Self {
            checkers: Arc::clone(&self.checkers),
            version: self.version.clone(),
        }
    }
}

/// Database health checker
pub struct DatabaseHealthChecker {
    #[allow(dead_code)]
    name: String,
}

impl DatabaseHealthChecker {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait::async_trait]
impl HealthChecker for DatabaseHealthChecker {
    async fn check_health(&self) -> ComponentHealth {
        // This is a stub - in real implementation, would check DB connection
        ComponentHealth::healthy().with_response_time(5)
    }
}

/// Scheduler health checker
pub struct SchedulerHealthChecker {
    last_execution: Arc<RwLock<Option<SystemTime>>>,
}

impl SchedulerHealthChecker {
    pub fn new() -> Self {
        Self {
            last_execution: Arc::new(RwLock::new(None)),
        }
    }

    pub fn record_execution(&self) {
        let mut last = self.last_execution.write().unwrap();
        *last = Some(SystemTime::now());
    }
}

impl Default for SchedulerHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HealthChecker for SchedulerHealthChecker {
    async fn check_health(&self) -> ComponentHealth {
        let last_exec = *self.last_execution.read().unwrap();

        match last_exec {
            Some(time) => match SystemTime::now().duration_since(time) {
                Ok(duration) => {
                    if duration > Duration::from_secs(300) {
                        ComponentHealth::degraded("No task execution in last 5 minutes")
                    } else {
                        ComponentHealth::healthy()
                    }
                }
                Err(_) => ComponentHealth::degraded("Clock skew detected"),
            },
            None => ComponentHealth::degraded("Scheduler has not executed any tasks"),
        }
    }
}

/// Search engine health checker
pub struct SearchHealthChecker;

impl SearchHealthChecker {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SearchHealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HealthChecker for SearchHealthChecker {
    async fn check_health(&self) -> ComponentHealth {
        // This is a stub - in real implementation, would check search engine connectivity
        ComponentHealth::healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status() {
        assert_eq!(HealthStatus::Healthy.http_status(), 200);
        assert_eq!(HealthStatus::Degraded.http_status(), 200);
        assert_eq!(HealthStatus::Unhealthy.http_status(), 503);
    }

    #[test]
    fn test_component_health() {
        let healthy = ComponentHealth::healthy();
        assert_eq!(healthy.status, HealthStatus::Healthy);
        assert!(healthy.message.is_none());

        let degraded = ComponentHealth::degraded("Low memory");
        assert_eq!(degraded.status, HealthStatus::Degraded);
        assert_eq!(degraded.message, Some("Low memory".to_string()));

        let unhealthy = ComponentHealth::unhealthy("Database down");
        assert_eq!(unhealthy.status, HealthStatus::Unhealthy);
        assert_eq!(unhealthy.message, Some("Database down".to_string()));
    }

    #[test]
    fn test_health_check() {
        let mut check = HealthCheck::new("1.0.0");
        assert_eq!(check.status, HealthStatus::Healthy);

        check.add_component("database", ComponentHealth::healthy());
        assert_eq!(check.status, HealthStatus::Healthy);

        check.add_component("cache", ComponentHealth::degraded("High latency"));
        assert_eq!(check.status, HealthStatus::Degraded);

        check.add_component("scheduler", ComponentHealth::unhealthy("Not running"));
        assert_eq!(check.status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_registry() {
        let registry = HealthRegistry::new("1.0.0");

        registry.register("database", Arc::new(DatabaseHealthChecker::new("test_db")));
        registry.register("search", Arc::new(SearchHealthChecker::new()));

        let health = registry.check_all().await;
        assert_eq!(health.components.len(), 2);
        assert!(health.components.contains_key("database"));
        assert!(health.components.contains_key("search"));
    }

    #[tokio::test]
    async fn test_scheduler_health_checker() {
        let checker = SchedulerHealthChecker::new();

        // Initially should be degraded (no executions)
        let health = checker.check_health().await;
        assert_eq!(health.status, HealthStatus::Degraded);

        // After recording execution, should be healthy
        checker.record_execution();
        let health = checker.check_health().await;
        assert_eq!(health.status, HealthStatus::Healthy);
    }
}
