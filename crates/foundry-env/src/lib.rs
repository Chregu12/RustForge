//! # Foundry Environment Management
//!
//! Environment variable validation and management for Foundry applications.
//!
//! ## Features
//!
//! - Validate required environment variables
//! - Type checking for environment values
//! - .env file validation
//! - Environment reload support
//! - Auto-fix common issues

pub mod commands;
pub mod validator;

pub use commands::{EnvReloadCommand, EnvValidateCommand};
pub use validator::{EnvRule, EnvValidator, ValidationResult};

use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Load environment from .env file
pub fn load_env(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(path)?;
    let mut vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            vars.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(vars)
}

/// Reload environment variables
pub fn reload_env(path: &Path) -> Result<usize> {
    let vars = load_env(path)?;
    let count = vars.len();

    for (key, value) in vars {
        env::set_var(key, value);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_env() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        fs::write(&env_path, "KEY1=value1\nKEY2=value2\n# Comment\n").unwrap();

        let vars = load_env(&env_path).unwrap();
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("KEY1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_reload_env() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        fs::write(&env_path, "TEST_KEY=test_value\n").unwrap();

        let count = reload_env(&env_path).unwrap();
        assert_eq!(count, 1);
        assert_eq!(env::var("TEST_KEY").unwrap(), "test_value");
    }
}
