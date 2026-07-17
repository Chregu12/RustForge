//! Dashboard data structures

use crate::alert::{Alert, AlertRegistry};
use crate::check::{CheckRegistry, CheckResult};
use crate::metrics::MetricsSnapshot;
use crate::monitor::{Event, Monitor};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub health: HealthSummary,
    pub metrics: MetricsSnapshot,
    pub alerts: Vec<AlertSummary>,
    pub recent_events: Vec<Event>,
    pub generated_at: DateTime<Utc>,
}

/// Health summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub status: String,
    pub checks: Vec<CheckSummary>,
}

/// Check summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckSummary {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: Option<u64>,
}

/// Alert summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSummary {
    pub name: String,
    pub level: String,
    pub description: Option<String>,
    pub enabled: bool,
}

impl Dashboard {
    /// Generate dashboard data
    pub async fn generate() -> Self {
        let check_results = CheckRegistry::global().run_all().await;
        let alerts = AlertRegistry::global().list();
        let metrics = crate::metrics::MetricsRegistry::global().snapshot();
        let recent_events = Monitor::global().recent(50);

        let overall_status = if check_results.iter().all(|(_, r)| r.status == crate::check::CheckStatus::Pass) {
            "healthy"
        } else if check_results.iter().any(|(_, r)| r.status == crate::check::CheckStatus::Fail) {
            "unhealthy"
        } else {
            "degraded"
        };

        Self {
            health: HealthSummary {
                status: overall_status.to_string(),
                checks: check_results
                    .into_iter()
                    .map(|(name, result)| CheckSummary {
                        name,
                        status: format!("{:?}", result.status).to_lowercase(),
                        message: result.message,
                        duration_ms: result.duration_ms,
                    })
                    .collect(),
            },
            metrics,
            alerts: alerts
                .into_iter()
                .map(|a| AlertSummary {
                    name: a.name,
                    level: format!("{:?}", a.level).to_lowercase(),
                    description: a.description,
                    enabled: a.enabled,
                })
                .collect(),
            recent_events,
            generated_at: Utc::now(),
        }
    }
}

impl From<(String, CheckResult)> for CheckSummary {
    fn from((name, result): (String, CheckResult)) -> Self {
        Self {
            name,
            status: format!("{:?}", result.status).to_lowercase(),
            message: result.message,
            duration_ms: result.duration_ms,
        }
    }
}

impl From<Alert> for AlertSummary {
    fn from(alert: Alert) -> Self {
        Self {
            name: alert.name,
            level: format!("{:?}", alert.level).to_lowercase(),
            description: alert.description,
            enabled: alert.enabled,
        }
    }
}
