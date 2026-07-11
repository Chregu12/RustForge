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
use std::path::Path;

/// Load environment from a `.env` file into a [`HashMap`].
///
/// Uses [`dotenvy`] internally so it correctly handles:
/// - single- and double-quoted values (`KEY="hello world"`)
/// - `export KEY=value` prefixes
/// - inline comments
/// - backslash continuations
///
/// Variables are **not** written to the process environment; call
/// [`reload_env`] if you want that.
pub fn load_env(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }

    let iter = dotenvy::from_path_iter(path)
        .map_err(|e| anyhow::anyhow!("Failed to open .env file '{}': {}", path.display(), e))?;

    let mut vars = HashMap::new();
    for item in iter {
        let (key, value) = item.map_err(|e| {
            anyhow::anyhow!("Failed to parse .env file '{}': {}", path.display(), e)
        })?;
        vars.insert(key, value);
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
    use std::fs;
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

    /// The naive line-splitter could not handle quoted values; dotenvy can.
    #[test]
    fn test_load_env_quoted_value() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        // Double-quoted value with spaces — naive split_once('=') would leave
        // the quotes in the value; dotenvy strips them correctly.
        fs::write(
            &env_path,
            "GREETING=\"hello world\"\nEXPORT_KEY=exported\n",
        )
        .unwrap();

        let vars = load_env(&env_path).unwrap();
        assert_eq!(
            vars.get("GREETING").map(String::as_str),
            Some("hello world"),
            "dotenvy must strip surrounding quotes"
        );
        assert_eq!(
            vars.get("EXPORT_KEY").map(String::as_str),
            Some("exported")
        );
    }

    /// export-prefixed entries must be parsed correctly.
    #[test]
    fn test_load_env_export_prefix() {
        let temp_dir = TempDir::new().unwrap();
        let env_path = temp_dir.path().join(".env");

        fs::write(&env_path, "export MY_SECRET=supersecret\n").unwrap();

        let vars = load_env(&env_path).unwrap();
        assert_eq!(
            vars.get("MY_SECRET").map(String::as_str),
            Some("supersecret"),
            "export prefix must be stripped by dotenvy"
        );
    }
}
