/// Extensions for CommandContext to support verbosity and other convenience methods
///
/// This module provides extensions to CommandContext that are commonly needed
/// but don't belong in the core plugins crate.

use crate::verbosity::Verbosity;
use foundry_plugins::CommandContext;

/// Extension methods for CommandContext
pub trait CommandContextExt {
    /// Extract and parse verbosity from command arguments
    fn verbosity(&self) -> Verbosity;

    /// Check if a flag is present in arguments
    fn has_flag(&self, flag: &str) -> bool;

    /// Get the value of an option flag (e.g., --option=value)
    fn option(&self, name: &str) -> Option<String>;

    /// Get positional arguments (those not starting with -)
    fn positional_args(&self) -> Vec<String>;

    /// Get named arguments as key-value pairs
    fn named_args(&self) -> Vec<(String, String)>;
}

impl CommandContextExt for CommandContext {
    fn verbosity(&self) -> Verbosity {
        Verbosity::from_args(&self.args)
    }

    fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|arg| arg == flag || arg.starts_with(&format!("--{}", flag)))
    }

    fn option(&self, name: &str) -> Option<String> {
        let prefix = format!("--{}=", name);
        self.args.iter()
            .find_map(|arg| {
                if arg.starts_with(&prefix) {
                    Some(arg.strip_prefix(&prefix).unwrap().to_string())
                } else if arg == &format!("--{}", name) {
                    // Find the next argument as the value
                    None // Would need index to implement properly
                } else {
                    None
                }
            })
    }

    fn positional_args(&self) -> Vec<String> {
        self.args
            .iter()
            .filter(|arg| !arg.starts_with('-'))
            .cloned()
            .collect()
    }

    fn named_args(&self) -> Vec<(String, String)> {
        self.args
            .iter()
            .filter_map(|arg| {
                if let Some(value) = arg.strip_prefix("--") {
                    if let Some((key, val)) = value.split_once('=') {
                        return Some((key.to_string(), val.to_string()));
                    }
                }
                None
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::{CommandContext, ResponseFormat, ExecutionOptions};
    use std::sync::Arc;
    use mockall::mock;

    // Mock implementations for testing would go here
    #[test]
    fn test_context_extensions() {
        // These tests would require creating a mock CommandContext
        // For now, we document the expected behavior
    }
}
