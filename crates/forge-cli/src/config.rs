#![allow(dead_code)] // utility helpers reserved for CLI commands, not all consumed yet
//! CLI configuration from .forge.toml
//!
//! This module handles reading and parsing .forge.toml configuration files
//! for customizing CLI behavior.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// CLI configuration from .forge.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    #[serde(default)]
    pub cli: CliConfig,

    #[serde(default)]
    pub aliases: HashMap<String, String>,

    #[serde(default)]
    pub defaults: HashMap<String, DefaultValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CliConfig {
    pub interactive: bool,
    pub color: bool,
    pub progress: bool,
    pub verbose: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            interactive: true,
            color: true,
            progress: true,
            verbose: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DefaultValue {
    Bool(bool),
    String(String),
    Number(i64),
}

impl ForgeConfig {
    /// Load configuration from .forge.toml in current directory
    pub fn load() -> Result<Self> {
        Self::load_from_path(".forge.toml")
    }

    /// Load configuration from a specific path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;

        let config: Self =
            toml::from_str(&content).context("Failed to parse .forge.toml configuration")?;

        Ok(config)
    }

    /// Save configuration to .forge.toml
    pub fn save(&self) -> Result<()> {
        self.save_to_path(".forge.toml")
    }

    /// Save configuration to a specific path
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize configuration")?;

        fs::write(path.as_ref(), content).context(format!(
            "Failed to write config file: {}",
            path.as_ref().display()
        ))?;

        Ok(())
    }

    /// Get a default value for a command option
    pub fn get_default(&self, key: &str) -> Option<&DefaultValue> {
        self.defaults.get(key)
    }

    /// Get a default boolean value
    pub fn get_default_bool(&self, key: &str) -> Option<bool> {
        match self.get_default(key) {
            Some(DefaultValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Get a default string value
    pub fn get_default_string(&self, key: &str) -> Option<&str> {
        match self.get_default(key) {
            Some(DefaultValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Create an example configuration file
    pub fn example() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("fresh".to_string(), "migrate:fresh --seed".to_string());
        aliases.insert("mfs".to_string(), "migrate:fresh --seed".to_string());

        let mut defaults = HashMap::new();
        defaults.insert("make:model.migration".to_string(), DefaultValue::Bool(true));
        defaults.insert("make:model.factory".to_string(), DefaultValue::Bool(true));
        defaults.insert(
            "make:model.timestamps".to_string(),
            DefaultValue::Bool(true),
        );
        defaults.insert("serve.port".to_string(), DefaultValue::Number(8000));

        Self {
            cli: CliConfig::default(),
            aliases,
            defaults,
        }
    }
}

/// Create a default .forge.toml file in the current directory
pub fn create_default_config() -> Result<()> {
    let config = ForgeConfig::example();
    config.save()?;
    Ok(())
}

/// Display current configuration
pub fn display_config(config: &ForgeConfig) {
    use colored::*;

    println!();
    println!("{}", "Current Configuration:".bold().cyan());
    println!();

    // CLI settings
    println!("{}", "[cli]".yellow().bold());
    println!(
        "  interactive = {}",
        format!("{}", config.cli.interactive).green()
    );
    println!("  color = {}", format!("{}", config.cli.color).green());
    println!(
        "  progress = {}",
        format!("{}", config.cli.progress).green()
    );
    println!("  verbose = {}", format!("{}", config.cli.verbose).green());
    println!();

    // Aliases
    if !config.aliases.is_empty() {
        println!("{}", "[aliases]".yellow().bold());
        for (alias, command) in &config.aliases {
            println!(
                "  {} = {}",
                alias.cyan(),
                format!("\"{}\"", command).green()
            );
        }
        println!();
    }

    // Defaults
    if !config.defaults.is_empty() {
        println!("{}", "[defaults]".yellow().bold());
        for (key, value) in &config.defaults {
            let value_str = match value {
                DefaultValue::Bool(b) => b.to_string(),
                DefaultValue::String(s) => format!("\"{}\"", s),
                DefaultValue::Number(n) => n.to_string(),
            };
            println!("  {} = {}", key.cyan(), value_str.green());
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ForgeConfig::default();
        assert!(config.cli.interactive);
        assert!(config.cli.color);
        assert!(config.cli.progress);
        assert!(!config.cli.verbose);
    }

    #[test]
    fn test_cli_config_default() {
        let config = CliConfig::default();
        assert!(config.interactive);
        assert!(config.color);
        assert!(config.progress);
        assert!(!config.verbose);
    }

    #[test]
    fn test_example_config() {
        let config = ForgeConfig::example();
        assert!(!config.aliases.is_empty());
        assert!(!config.defaults.is_empty());
    }

    #[test]
    fn test_get_default_bool() {
        let config = ForgeConfig::example();
        assert_eq!(config.get_default_bool("make:model.migration"), Some(true));
    }

    #[test]
    fn test_get_default_string() {
        let mut config = ForgeConfig::default();
        config.defaults.insert(
            "test.key".to_string(),
            DefaultValue::String("value".to_string()),
        );
        assert_eq!(config.get_default_string("test.key"), Some("value"));
    }

    #[test]
    fn test_save_and_load_config() -> Result<()> {
        let dir = TempDir::new()?;
        let config_path = dir.path().join("test.toml");

        let config = ForgeConfig::example();
        config.save_to_path(&config_path)?;

        let loaded = ForgeConfig::load_from_path(&config_path)?;
        assert_eq!(loaded.cli.interactive, config.cli.interactive);
        assert_eq!(loaded.aliases.len(), config.aliases.len());

        Ok(())
    }

    #[test]
    fn test_load_nonexistent_config() -> Result<()> {
        let config = ForgeConfig::load_from_path("nonexistent.toml")?;
        // Should return default config
        assert!(config.cli.interactive);
        Ok(())
    }

    #[test]
    fn test_default_value_bool() {
        let val = DefaultValue::Bool(true);
        match val {
            DefaultValue::Bool(b) => assert!(b),
            _ => panic!("Expected Bool variant"),
        }
    }

    #[test]
    fn test_default_value_string() {
        let val = DefaultValue::String("test".to_string());
        match val {
            DefaultValue::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected String variant"),
        }
    }

    #[test]
    fn test_default_value_number() {
        let val = DefaultValue::Number(42);
        match val {
            DefaultValue::Number(n) => assert_eq!(n, 42),
            _ => panic!("Expected Number variant"),
        }
    }
}
