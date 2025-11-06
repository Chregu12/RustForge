//! Health check system for monitoring application status

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Name of the component being checked
    pub name: String,

    /// Health state
    pub status: HealthState,

    /// Optional status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Check duration in milliseconds
    pub duration_ms: u64,

    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,

    /// Additional metadata
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HealthCheck {
    /// Create a healthy check result
    pub fn healthy(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Healthy,
            message: None,
            duration_ms: duration.as_millis() as u64,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a degraded check result
    pub fn degraded(name: impl Into<String>, message: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Degraded,
            message: Some(message.into()),
            duration_ms: duration.as_millis() as u64,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create an unhealthy check result
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Unhealthy,
            message: Some(message.into()),
            duration_ms: duration.as_millis() as u64,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the health check
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Complete health status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall status
    pub status: HealthState,

    /// Individual component checks
    pub checks: Vec<HealthCheck>,

    /// Application version
    pub version: String,

    /// Uptime in seconds
    pub uptime_seconds: u64,

    /// Timestamp of the health check
    pub timestamp: DateTime<Utc>,
}

impl HealthStatus {
    /// Create a new health status
    pub fn new(checks: Vec<HealthCheck>, start_time: Instant) -> Self {
        let status = Self::determine_overall_status(&checks);
        let uptime = start_time.elapsed();

        Self {
            status,
            checks,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime.as_secs(),
            timestamp: Utc::now(),
        }
    }

    /// Determine overall status from individual checks
    fn determine_overall_status(checks: &[HealthCheck]) -> HealthState {
        if checks.iter().any(|c| c.status == HealthState::Unhealthy) {
            HealthState::Unhealthy
        } else if checks.iter().any(|c| c.status == HealthState::Degraded) {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        }
    }
}

/// Trait for implementing custom health checks
#[async_trait]
pub trait HealthChecker: Send + Sync {
    /// Perform the health check
    async fn check(&self) -> HealthCheck;

    /// Get the name of this health checker
    fn name(&self) -> &str;
}

/// Database health checker
pub struct DatabaseHealthChecker {
    name: String,
}

impl DatabaseHealthChecker {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}

#[async_trait]
impl HealthChecker for DatabaseHealthChecker {
    async fn check(&self) -> HealthCheck {
        let start = Instant::now();

        // TODO: Implement actual database check
        // For now, simulate a check
        tokio::time::sleep(Duration::from_millis(10)).await;

        let duration = start.elapsed();
        HealthCheck::healthy(&self.name, duration)
            .with_metadata("type", serde_json::json!("database"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Redis/Cache health checker
pub struct CacheHealthChecker {
    name: String,
}

impl CacheHealthChecker {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}

#[async_trait]
impl HealthChecker for CacheHealthChecker {
    async fn check(&self) -> HealthCheck {
        let start = Instant::now();

        // TODO: Implement actual cache check
        tokio::time::sleep(Duration::from_millis(5)).await;

        let duration = start.elapsed();
        HealthCheck::healthy(&self.name, duration)
            .with_metadata("type", serde_json::json!("cache"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Queue health checker
pub struct QueueHealthChecker {
    name: String,
}

impl QueueHealthChecker {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }
}

#[async_trait]
impl HealthChecker for QueueHealthChecker {
    async fn check(&self) -> HealthCheck {
        let start = Instant::now();

        // TODO: Implement actual queue check
        tokio::time::sleep(Duration::from_millis(5)).await;

        let duration = start.elapsed();
        HealthCheck::healthy(&self.name, duration)
            .with_metadata("type", serde_json::json!("queue"))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Health check registry
pub struct HealthCheckRegistry {
    checkers: Arc<RwLock<Vec<Arc<dyn HealthChecker>>>>,
    start_time: Instant,
}

impl HealthCheckRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            checkers: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Register a health checker
    pub async fn register(&self, checker: Arc<dyn HealthChecker>) {
        let mut checkers = self.checkers.write().await;
        checkers.push(checker);
    }

    /// Run all health checks
    pub async fn check_health(&self) -> HealthStatus {
        let checkers = self.checkers.read().await;
        let mut checks = Vec::new();

        for checker in checkers.iter() {
            let check = checker.check().await;
            checks.push(check);
        }

        HealthStatus::new(checks, self.start_time)
    }
}

impl Default for HealthCheckRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_creation() {
        let check = HealthCheck::healthy("test", Duration::from_millis(100));
        assert_eq!(check.name, "test");
        assert_eq!(check.status, HealthState::Healthy);
        assert_eq!(check.duration_ms, 100);
    }

    #[test]
    fn test_health_status_determination() {
        let checks = vec![
            HealthCheck::healthy("db", Duration::from_millis(10)),
            HealthCheck::healthy("cache", Duration::from_millis(5)),
        ];

        let status = HealthStatus::new(checks, Instant::now());
        assert_eq!(status.status, HealthState::Healthy);
    }

    #[test]
    fn test_unhealthy_status_propagation() {
        let checks = vec![
            HealthCheck::healthy("db", Duration::from_millis(10)),
            HealthCheck::unhealthy("cache", "Connection failed", Duration::from_millis(5)),
        ];

        let status = HealthStatus::new(checks, Instant::now());
        assert_eq!(status.status, HealthState::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_registry() {
        let registry = HealthCheckRegistry::new();
        registry
            .register(Arc::new(DatabaseHealthChecker::new("test_db")))
            .await;

        let status = registry.check_health().await;
        assert_eq!(status.checks.len(), 1);
    }
}
