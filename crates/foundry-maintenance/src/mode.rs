//! Maintenance mode manager

use crate::config::{MaintenanceConfig, MaintenanceState};
use anyhow::{Context, Result};
use std::fs;

/// Maintenance mode manager
pub struct MaintenanceMode {
    config: MaintenanceConfig,
}

impl MaintenanceMode {
    /// Create a new maintenance mode manager
    pub fn new(config: MaintenanceConfig) -> Self {
        Self { config }
    }

    /// Check if maintenance mode is active
    pub fn is_active(&self) -> bool {
        self.config.file_path.exists()
    }

    /// Enable maintenance mode
    pub fn enable(&self) -> Result<()> {
        let state = MaintenanceState::new(
            self.config.message.clone(),
            self.config.secret.clone(),
        );

        let json = serde_json::to_string_pretty(&state)
            .context("Failed to serialize maintenance state")?;

        fs::write(&self.config.file_path, json)
            .context("Failed to write maintenance file")?;

        Ok(())
    }

    /// Enable with custom retry-after
    pub fn enable_with_retry(&self, retry_after_seconds: u64) -> Result<()> {
        let state = MaintenanceState::new(
            self.config.message.clone(),
            self.config.secret.clone(),
        )
        .with_retry_after(retry_after_seconds);

        let json = serde_json::to_string_pretty(&state)
            .context("Failed to serialize maintenance state")?;

        fs::write(&self.config.file_path, json)
            .context("Failed to write maintenance file")?;

        Ok(())
    }

    /// Disable maintenance mode
    pub fn disable(&self) -> Result<()> {
        if self.config.file_path.exists() {
            fs::remove_file(&self.config.file_path)
                .context("Failed to remove maintenance file")?;
        }
        Ok(())
    }

    /// Get current maintenance state
    pub fn get_state(&self) -> Result<Option<MaintenanceState>> {
        if !self.is_active() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.config.file_path)
            .context("Failed to read maintenance file")?;

        let state: MaintenanceState = serde_json::from_str(&content)
            .context("Failed to parse maintenance state")?;

        Ok(Some(state))
    }

    /// Update maintenance message
    pub fn update_message(&self, message: String) -> Result<()> {
        if let Some(mut state) = self.get_state()? {
            state.message = Some(message);

            let json = serde_json::to_string_pretty(&state)
                .context("Failed to serialize maintenance state")?;

            fs::write(&self.config.file_path, json)
                .context("Failed to write maintenance file")?;
        }
        Ok(())
    }

    /// Get config
    pub fn config(&self) -> &MaintenanceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_enable_disable() {
        let temp_dir = TempDir::new().unwrap();
        let config = MaintenanceConfig {
            file_path: temp_dir.path().join(".maintenance"),
            message: Some("Test message".to_string()),
            secret: None,
        };

        let mode = MaintenanceMode::new(config);

        assert!(!mode.is_active());

        mode.enable().unwrap();
        assert!(mode.is_active());

        mode.disable().unwrap();
        assert!(!mode.is_active());
    }

    #[test]
    fn test_enable_with_secret() {
        let temp_dir = TempDir::new().unwrap();
        let config = MaintenanceConfig {
            file_path: temp_dir.path().join(".maintenance"),
            message: Some("Maintenance".to_string()),
            secret: Some("secret123".to_string()),
        };

        let mode = MaintenanceMode::new(config);
        mode.enable().unwrap();

        let state = mode.get_state().unwrap();
        assert!(state.is_some());

        let state = state.unwrap();
        assert_eq!(state.secret, Some("secret123".to_string()));
    }

    #[test]
    fn test_enable_with_retry() {
        let temp_dir = TempDir::new().unwrap();
        let config = MaintenanceConfig {
            file_path: temp_dir.path().join(".maintenance"),
            message: None,
            secret: None,
        };

        let mode = MaintenanceMode::new(config);
        mode.enable_with_retry(3600).unwrap();

        let state = mode.get_state().unwrap().unwrap();
        assert_eq!(state.retry_after, Some(3600));
    }

    #[test]
    fn test_update_message() {
        let temp_dir = TempDir::new().unwrap();
        let config = MaintenanceConfig {
            file_path: temp_dir.path().join(".maintenance"),
            message: Some("Initial".to_string()),
            secret: None,
        };

        let mode = MaintenanceMode::new(config);
        mode.enable().unwrap();

        mode.update_message("Updated message".to_string()).unwrap();

        let state = mode.get_state().unwrap().unwrap();
        assert_eq!(state.message, Some("Updated message".to_string()));
    }

    #[test]
    fn test_get_state_when_inactive() {
        let temp_dir = TempDir::new().unwrap();
        let config = MaintenanceConfig {
            file_path: temp_dir.path().join(".maintenance"),
            message: None,
            secret: None,
        };

        let mode = MaintenanceMode::new(config);
        let state = mode.get_state().unwrap();
        assert!(state.is_none());
    }
}
