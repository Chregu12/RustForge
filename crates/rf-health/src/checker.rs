//! Health check trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Health check status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// Service is healthy
    Healthy,
    /// Service is degraded but operational
    Degraded,
    /// Service is unhealthy
    Unhealthy,
}

impl HealthStatus {
    /// Check if status is healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Check if status is degraded
    pub fn is_degraded(&self) -> bool {
        matches!(self, HealthStatus::Degraded)
    }

    /// Check if status is unhealthy
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, HealthStatus::Unhealthy)
    }

    /// Get HTTP status code for liveness probes and general health endpoints.
    ///
    /// `Degraded` returns 200 here because the process is still alive and
    /// operational — liveness probes should only fail when the process must be
    /// restarted (Unhealthy).  For readiness probes (k8s traffic gating) use
    /// [`readiness_http_status`](HealthStatus::readiness_http_status) instead.
    pub fn http_status(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 200,
            HealthStatus::Unhealthy => 503,
        }
    }

    /// Get HTTP status code for readiness probes.
    ///
    /// Unlike [`http_status`](HealthStatus::http_status), `Degraded` maps to
    /// **503** here so that load balancers and Kubernetes readiness probes can
    /// gate traffic away from a degraded pod.  This is the correct code to use
    /// for `/health/ready` endpoints.
    pub fn readiness_http_status(&self) -> u16 {
        match self {
            HealthStatus::Healthy => 200,
            HealthStatus::Degraded => 503,
            HealthStatus::Unhealthy => 503,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Optional metadata
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CheckResult {
    /// Create healthy result
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create degraded result
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create unhealthy result
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Health check trait
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Get check name
    fn name(&self) -> &str;

    /// Perform health check
    async fn check(&self) -> CheckResult;

    /// Check if this is a liveness check (for Kubernetes)
    fn is_liveness(&self) -> bool {
        false
    }

    /// Check if this is a readiness check (for Kubernetes)
    fn is_readiness(&self) -> bool {
        true
    }
}

/// Composite health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Overall status
    pub status: HealthStatus,
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthResponse {
    /// Create from check results
    pub fn from_checks(checks: Vec<CheckResult>) -> Self {
        // Determine overall status:
        // - If any check is unhealthy, overall is unhealthy
        // - If any check is degraded, overall is degraded
        // - Otherwise, overall is healthy
        let status = if checks.iter().any(|c| c.status.is_unhealthy()) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| c.status.is_degraded()) {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        Self {
            status,
            checks,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get HTTP status code for liveness probes and general health endpoints.
    ///
    /// `Degraded` returns 200 (process is alive).  Use
    /// [`readiness_http_status`](HealthResponse::readiness_http_status) for
    /// readiness probes that need to gate traffic away from degraded pods.
    pub fn http_status(&self) -> u16 {
        self.status.http_status()
    }

    /// Get HTTP status code for readiness probes.
    ///
    /// Returns 503 for both `Degraded` and `Unhealthy` so load balancers and
    /// Kubernetes readiness probes can remove the pod from rotation when the
    /// service is not fully operational.
    pub fn readiness_http_status(&self) -> u16 {
        self.status.readiness_http_status()
    }
}
