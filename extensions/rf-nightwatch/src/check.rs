//! Health check system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

type CheckHandler = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = CheckResult> + Send>> + Send + Sync>;

static CHECK_REGISTRY: OnceLock<Arc<CheckRegistry>> = OnceLock::new();

/// Check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

/// Check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub status: CheckStatus,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub checked_at: DateTime<Utc>,
}

impl CheckResult {
    /// Create a passing check result
    pub fn pass(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Pass,
            message: Some(message.into()),
            duration_ms: None,
            details: None,
            checked_at: Utc::now(),
        }
    }

    /// Create a warning check result
    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Warn,
            message: Some(message.into()),
            duration_ms: None,
            details: None,
            checked_at: Utc::now(),
        }
    }

    /// Create a failing check result
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Fail,
            message: Some(message.into()),
            duration_ms: None,
            details: None,
            checked_at: Utc::now(),
        }
    }

    /// Add details to the result
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Set the duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = Some(duration.as_millis() as u64);
        self
    }
}

/// A health check
#[derive(Clone)]
pub struct Check {
    /// Check name
    pub name: String,
    /// Check description
    pub description: Option<String>,
    /// Check handler
    handler: CheckHandler,
    /// Last result
    #[allow(dead_code)]
    last_result: Arc<RwLock<Option<CheckResult>>>,
}

impl Check {
    /// Run the check
    pub async fn run(&self) -> CheckResult {
        let start = std::time::Instant::now();
        let mut result = (self.handler)().await;
        result.duration_ms = Some(start.elapsed().as_millis() as u64);
        result
    }
}

/// Check builder
pub struct CheckBuilder {
    name: String,
    description: Option<String>,
    handler: CheckHandler,
    #[allow(dead_code)]
    timeout: Duration,
}

impl CheckBuilder {
    /// Create a new check builder
    pub fn new<F>(name: &str, handler: F) -> Self
    where
        F: Fn() -> Pin<Box<dyn Future<Output = CheckResult> + Send>> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            description: None,
            handler: Arc::new(handler),
            timeout: Duration::from_secs(30),
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Register the check
    pub fn register(self) {
        let check = Check {
            name: self.name.clone(),
            description: self.description,
            handler: self.handler,
            last_result: Arc::new(RwLock::new(None)),
        };

        CheckRegistry::global().register(check);
    }
}

/// Check registry
pub struct CheckRegistry {
    checks: RwLock<HashMap<String, Check>>,
}

impl CheckRegistry {
    /// Create a new check registry
    pub fn new() -> Self {
        Self {
            checks: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global check registry
    pub fn global() -> Arc<Self> {
        CHECK_REGISTRY.get_or_init(|| Arc::new(Self::new())).clone()
    }

    /// Register a check
    pub fn register(&self, check: Check) {
        let mut checks = self.checks.write().unwrap();
        checks.insert(check.name.clone(), check);
    }

    /// Get a check by name
    pub fn get(&self, name: &str) -> Option<Check> {
        let checks = self.checks.read().unwrap();
        checks.get(name).cloned()
    }

    /// List all checks
    pub fn list(&self) -> Vec<Check> {
        let checks = self.checks.read().unwrap();
        checks.values().cloned().collect()
    }

    /// Run all checks
    pub async fn run_all(&self) -> Vec<(String, CheckResult)> {
        let checks = self.list();
        let mut results = Vec::new();

        for check in checks {
            let result = check.run().await;
            results.push((check.name.clone(), result));
        }

        results
    }

    /// Run a specific check
    pub async fn run(&self, name: &str) -> Option<CheckResult> {
        if let Some(check) = self.get(name) {
            Some(check.run().await)
        } else {
            None
        }
    }
}

impl Default for CheckRegistry {
    fn default() -> Self {
        Self::new()
    }
}
