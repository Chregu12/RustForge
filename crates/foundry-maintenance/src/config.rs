//! Maintenance mode configuration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for maintenance mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceConfig {
    /// Path to the maintenance flag file
    pub file_path: PathBuf,
    /// Custom maintenance message
    pub message: Option<String>,
    /// Secret token for bypass
    pub secret: Option<String>,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            file_path: PathBuf::from(".maintenance"),
            message: Some("We are currently down for maintenance. Please check back soon.".to_string()),
            secret: None,
        }
    }
}

/// Maintenance state stored in the flag file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceState {
    /// When maintenance mode was enabled
    pub enabled_at: String,
    /// Custom message
    pub message: Option<String>,
    /// Secret for bypass (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    /// Retry-After header value in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u64>,
}

impl MaintenanceState {
    /// Create a new maintenance state
    pub fn new(message: Option<String>, secret: Option<String>) -> Self {
        Self {
            enabled_at: chrono::Utc::now().to_rfc3339(),
            message,
            secret,
            retry_after: None,
        }
    }

    /// Set retry after duration
    pub fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self
    }

    /// Get display message
    pub fn display_message(&self) -> String {
        self.message
            .clone()
            .unwrap_or_else(|| "Service Temporarily Unavailable".to_string())
    }

    /// Check if secret matches
    pub fn verify_secret(&self, provided: &str) -> bool {
        if let Some(secret) = &self.secret {
            secret == provided
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = MaintenanceConfig::default();
        assert_eq!(config.file_path, PathBuf::from(".maintenance"));
        assert!(config.message.is_some());
        assert!(config.secret.is_none());
    }

    #[test]
    fn test_state_new() {
        let state = MaintenanceState::new(
            Some("Custom message".to_string()),
            Some("secret123".to_string()),
        );

        assert!(state.message.is_some());
        assert!(state.secret.is_some());
        assert!(!state.enabled_at.is_empty());
    }

    #[test]
    fn test_state_with_retry_after() {
        let state = MaintenanceState::new(None, None).with_retry_after(3600);

        assert_eq!(state.retry_after, Some(3600));
    }

    #[test]
    fn test_state_display_message() {
        let state1 = MaintenanceState::new(Some("Custom".to_string()), None);
        assert_eq!(state1.display_message(), "Custom");

        let state2 = MaintenanceState::new(None, None);
        assert_eq!(state2.display_message(), "Service Temporarily Unavailable");
    }

    #[test]
    fn test_state_verify_secret() {
        let state = MaintenanceState::new(None, Some("mysecret".to_string()));

        assert!(state.verify_secret("mysecret"));
        assert!(!state.verify_secret("wrongsecret"));
    }

    #[test]
    fn test_state_serialize() {
        let state = MaintenanceState::new(
            Some("Test".to_string()),
            Some("secret".to_string()),
        );

        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("secret"));
    }
}
