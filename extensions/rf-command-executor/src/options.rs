//! Command execution options and configuration

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Execution mode for commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute command immediately
    Immediate,
    /// Queue command for later execution
    Queued,
}

/// Options for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandOptions {
    /// Execution mode (immediate or queued)
    pub mode: ExecutionMode,

    /// Named arguments (--key=value)
    #[serde(default)]
    pub named_args: HashMap<String, String>,

    /// Array arguments (--tags=tag1,tag2,tag3)
    #[serde(default)]
    pub array_args: HashMap<String, Vec<String>>,

    /// Boolean flags (--force, --verbose)
    #[serde(default)]
    pub flags: Vec<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: Value,

    /// Capture stdout
    #[serde(default = "default_true")]
    pub capture_output: bool,

    /// Capture stderr
    #[serde(default = "default_true")]
    pub capture_errors: bool,

    /// Dry run mode
    #[serde(default)]
    pub dry_run: bool,

    /// Force execution
    #[serde(default)]
    pub force: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Immediate,
            named_args: HashMap::new(),
            array_args: HashMap::new(),
            flags: Vec::new(),
            metadata: Value::Null,
            capture_output: true,
            capture_errors: true,
            dry_run: false,
            force: false,
        }
    }
}

impl CommandOptions {
    /// Create new command options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set execution mode
    pub fn mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Add a named argument
    pub fn arg<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.named_args.insert(key.into(), value.into());
        self
    }

    /// Add an array argument
    pub fn array_arg<K: Into<String>, V: Into<Vec<String>>>(mut self, key: K, values: V) -> Self {
        self.array_args.insert(key.into(), values.into());
        self
    }

    /// Add a boolean flag
    pub fn flag<S: Into<String>>(mut self, flag: S) -> Self {
        self.flags.push(flag.into());
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Enable output capture
    pub fn capture_output(mut self, capture: bool) -> Self {
        self.capture_output = capture;
        self
    }

    /// Enable error capture
    pub fn capture_errors(mut self, capture: bool) -> Self {
        self.capture_errors = capture;
        self
    }

    /// Enable dry run mode
    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Enable force mode
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Convert options to command arguments
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Add named arguments
        for (key, value) in &self.named_args {
            args.push(format!("--{}={}", key, value));
        }

        // Add array arguments
        for (key, values) in &self.array_args {
            args.push(format!("--{}={}", key, values.join(",")));
        }

        // Add flags
        for flag in &self.flags {
            args.push(format!("--{}", flag));
        }

        // Add dry-run flag if enabled
        if self.dry_run {
            args.push("--dry-run".to_string());
        }

        // Add force flag if enabled
        if self.force {
            args.push("--force".to_string());
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let opts = CommandOptions::default();
        assert_eq!(opts.mode, ExecutionMode::Immediate);
        assert!(opts.capture_output);
        assert!(opts.capture_errors);
        assert!(!opts.dry_run);
        assert!(!opts.force);
    }

    #[test]
    fn test_builder_pattern() {
        let opts = CommandOptions::new()
            .arg("name", "test")
            .flag("verbose")
            .dry_run(true);

        assert_eq!(opts.named_args.get("name"), Some(&"test".to_string()));
        assert!(opts.flags.contains(&"verbose".to_string()));
        assert!(opts.dry_run);
    }

    #[test]
    fn test_to_args() {
        let opts = CommandOptions::new()
            .arg("name", "test")
            .array_arg("tags", vec!["tag1".to_string(), "tag2".to_string()])
            .flag("verbose")
            .force(true);

        let args = opts.to_args();
        assert!(args.contains(&"--name=test".to_string()));
        assert!(args.contains(&"--tags=tag1,tag2".to_string()));
        assert!(args.contains(&"--verbose".to_string()));
        assert!(args.contains(&"--force".to_string()));
    }
}
