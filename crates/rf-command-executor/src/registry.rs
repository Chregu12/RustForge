//! Command registry for command execution

use crate::error::{ExecutionError, ExecutionResult};
use rf_plugins::DynCommand;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe command registry
#[derive(Clone)]
pub struct CommandRegistry {
    commands: Arc<RwLock<HashMap<String, DynCommand>>>,
}

impl CommandRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            commands: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a command
    pub fn register(&self, name: String, command: DynCommand) -> ExecutionResult<()> {
        self.commands
            .write()
            .map_err(|e| ExecutionError::Other(anyhow::anyhow!("Lock poisoned: {}", e)))?
            .insert(name, command);
        Ok(())
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> ExecutionResult<DynCommand> {
        self.commands
            .read()
            .map_err(|e| ExecutionError::Other(anyhow::anyhow!("Lock poisoned: {}", e)))?
            .get(name)
            .cloned()
            .ok_or_else(|| ExecutionError::CommandNotFound(name.to_string()))
    }

    /// Check if a command exists
    pub fn has(&self, name: &str) -> bool {
        self.commands
            .read()
            .map(|commands| commands.contains_key(name))
            .unwrap_or(false)
    }

    /// Get all registered command names
    pub fn command_names(&self) -> Vec<String> {
        self.commands
            .read()
            .map(|commands| commands.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get total number of registered commands
    pub fn count(&self) -> usize {
        self.commands
            .read()
            .map(|commands| commands.len())
            .unwrap_or(0)
    }

    /// Clear all registered commands
    pub fn clear(&self) -> ExecutionResult<()> {
        self.commands
            .write()
            .map_err(|e| ExecutionError::Other(anyhow::anyhow!("Lock poisoned: {}", e)))?
            .clear();
        Ok(())
    }

    /// Unregister a command
    pub fn unregister(&self, name: &str) -> ExecutionResult<Option<DynCommand>> {
        Ok(self
            .commands
            .write()
            .map_err(|e| ExecutionError::Other(anyhow::anyhow!("Lock poisoned: {}", e)))?
            .remove(name))
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Update tests to use new CommandDescriptor API
// #[cfg(test)]
// mod tests { ... }
