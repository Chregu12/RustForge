//! Alert system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

static ALERT_REGISTRY: OnceLock<Arc<AlertRegistry>> = OnceLock::new();

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert notification channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub channel: String,
    pub target: String,
}

/// A configured alert
#[derive(Clone)]
pub struct Alert {
    /// Alert name
    pub name: String,
    /// Alert description
    pub description: Option<String>,
    /// Alert level
    pub level: AlertLevel,
    /// Condition (as a description)
    #[allow(dead_code)]
    pub condition: String,
    /// Notification channels
    pub notifications: Vec<Notification>,
    /// Is alert enabled
    pub enabled: bool,
    /// Last triggered time
    #[allow(dead_code)]
    last_triggered: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl Alert {
    /// Trigger the alert
    pub fn trigger(&self, message: &str) {
        tracing::warn!(
            "Alert [{}] triggered: {} (level: {:?})",
            self.name,
            message,
            self.level
        );

        // Update last triggered time
        let mut last = self.last_triggered.write().unwrap();
        *last = Some(Utc::now());

        // In a real implementation, this would send notifications
        for notification in &self.notifications {
            tracing::info!(
                "Sending alert to {} via {}",
                notification.target,
                notification.channel
            );
        }
    }
}

/// Alert builder
pub struct AlertBuilder {
    name: String,
    description: Option<String>,
    level: AlertLevel,
    condition: String,
    notifications: Vec<Notification>,
}

impl AlertBuilder {
    /// Create a new alert builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            level: AlertLevel::Warning,
            condition: String::new(),
            notifications: Vec::new(),
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the alert level
    pub fn level(mut self, level: AlertLevel) -> Self {
        self.level = level;
        self
    }

    /// Set as info level
    pub fn info(mut self) -> Self {
        self.level = AlertLevel::Info;
        self
    }

    /// Set as warning level
    pub fn warning(mut self) -> Self {
        self.level = AlertLevel::Warning;
        self
    }

    /// Set as error level
    pub fn error(mut self) -> Self {
        self.level = AlertLevel::Error;
        self
    }

    /// Set as critical level
    pub fn critical(mut self) -> Self {
        self.level = AlertLevel::Critical;
        self
    }

    /// Set the condition (as a description)
    pub fn when(mut self, condition: &str) -> Self {
        self.condition = condition.to_string();
        self
    }

    /// Add a notification channel
    pub fn notify(mut self, channel: &str, target: &str) -> Self {
        self.notifications.push(Notification {
            channel: channel.to_string(),
            target: target.to_string(),
        });
        self
    }

    /// Register the alert
    pub fn register(self) {
        let alert = Alert {
            name: self.name.clone(),
            description: self.description,
            level: self.level,
            condition: self.condition,
            notifications: self.notifications,
            enabled: true,
            last_triggered: Arc::new(RwLock::new(None)),
        };

        AlertRegistry::global().register(alert);
    }
}

/// Alert registry
pub struct AlertRegistry {
    alerts: RwLock<HashMap<String, Alert>>,
}

impl AlertRegistry {
    /// Create a new alert registry
    pub fn new() -> Self {
        Self {
            alerts: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global alert registry
    pub fn global() -> Arc<Self> {
        ALERT_REGISTRY.get_or_init(|| Arc::new(Self::new())).clone()
    }

    /// Register an alert
    pub fn register(&self, alert: Alert) {
        let mut alerts = self.alerts.write().unwrap();
        alerts.insert(alert.name.clone(), alert);
    }

    /// Get an alert by name
    pub fn get(&self, name: &str) -> Option<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts.get(name).cloned()
    }

    /// List all alerts
    pub fn list(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts.values().cloned().collect()
    }

    /// Trigger an alert
    pub fn trigger(&self, name: &str, message: &str) {
        if let Some(alert) = self.get(name) {
            if alert.enabled {
                alert.trigger(message);
            }
        }
    }
}

impl Default for AlertRegistry {
    fn default() -> Self {
        Self::new()
    }
}
