//! # Foundry Maintenance Mode
//!
//! Provides maintenance mode functionality for Foundry applications, including:
//! - Activation and deactivation of maintenance mode
//! - Secret-based bypass for authorized users
//! - Middleware integration for request blocking
//! - Customizable maintenance messages
//!
//! ## Usage
//!
//! ```rust,no_run
//! use foundry_maintenance::{MaintenanceMode, MaintenanceConfig};
//!
//! let config = MaintenanceConfig {
//!     file_path: ".maintenance".into(),
//!     message: Some("We'll be back soon!".to_string()),
//!     secret: Some("secret-token".to_string()),
//! };
//!
//! let mut mode = MaintenanceMode::new(config);
//! mode.enable().unwrap();
//! ```

pub mod commands;
pub mod config;
pub mod middleware;
pub mod mode;

pub use commands::{AppDownCommand, AppUpCommand};
pub use config::{MaintenanceConfig, MaintenanceState};
pub use middleware::MaintenanceMiddleware;
pub use mode::MaintenanceMode;

use anyhow::Result;
use std::path::Path;

/// Check if maintenance mode is currently active
pub fn is_active(file_path: &Path) -> bool {
    file_path.exists()
}

/// Get current maintenance state
pub fn get_state(file_path: &Path) -> Result<Option<MaintenanceState>> {
    if !is_active(file_path) {
        return Ok(None);
    }

    let content = std::fs::read_to_string(file_path)?;
    let state: MaintenanceState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_active() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        assert!(!is_active(&file_path));

        std::fs::write(&file_path, "{}").unwrap();
        assert!(is_active(&file_path));
    }

    #[test]
    fn test_get_state() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        let result = get_state(&file_path).unwrap();
        assert!(result.is_none());

        let state = MaintenanceState {
            enabled_at: chrono::Utc::now().to_rfc3339(),
            message: Some("Test message".to_string()),
            secret: None,
            retry_after: None,
        };

        std::fs::write(&file_path, serde_json::to_string(&state).unwrap()).unwrap();

        let result = get_state(&file_path).unwrap();
        assert!(result.is_some());
    }
}
