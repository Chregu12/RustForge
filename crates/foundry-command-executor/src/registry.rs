//! Command registry for command execution

use crate::error::{ExecutionError, ExecutionResult};
use foundry_plugins::DynCommand;
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use foundry_domain::CommandDescriptor;
    use foundry_plugins::{CommandContext, CommandResult, FoundryCommand};

    struct TestCommand {
        descriptor: CommandDescriptor,
    }

    #[async_trait]
    impl FoundryCommand for TestCommand {
        fn descriptor(&self) -> &CommandDescriptor {
            &self.descriptor
        }

        async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, foundry_plugins::CommandError> {
            Ok(CommandResult::success("test"))
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let registry = CommandRegistry::new();
        let command = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test".to_string(),
                description: "Test command".to_string(),
                category: "test".to_string(),
                signature: "test".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        registry.register("test".to_string(), command.clone()).unwrap();

        assert!(registry.has("test"));
        assert!(registry.get("test").is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_registry_command_not_found() {
        let registry = CommandRegistry::new();
        let result = registry.get("nonexistent");

        assert!(result.is_err());
        match result {
            Err(ExecutionError::CommandNotFound(name)) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_registry_unregister() {
        let registry = CommandRegistry::new();
        let command = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test".to_string(),
                description: "Test command".to_string(),
                category: "test".to_string(),
                signature: "test".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        registry.register("test".to_string(), command).unwrap();
        assert_eq!(registry.count(), 1);

        let removed = registry.unregister("test").unwrap();
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
        assert!(!registry.has("test"));
    }

    #[test]
    fn test_registry_clear() {
        let registry = CommandRegistry::new();
        let command1 = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test1".to_string(),
                description: "Test command 1".to_string(),
                category: "test".to_string(),
                signature: "test1".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        let command2 = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test2".to_string(),
                description: "Test command 2".to_string(),
                category: "test".to_string(),
                signature: "test2".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        registry.register("test1".to_string(), command1).unwrap();
        registry.register("test2".to_string(), command2).unwrap();
        assert_eq!(registry.count(), 2);

        registry.clear().unwrap();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_registry_command_names() {
        let registry = CommandRegistry::new();
        let command1 = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test1".to_string(),
                description: "Test command 1".to_string(),
                category: "test".to_string(),
                signature: "test1".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        let command2 = Arc::new(TestCommand {
            descriptor: CommandDescriptor {
                name: "test2".to_string(),
                description: "Test command 2".to_string(),
                category: "test".to_string(),
                signature: "test2".to_string(),
                hidden: false,
            },
        }) as DynCommand;

        registry.register("test1".to_string(), command1).unwrap();
        registry.register("test2".to_string(), command2).unwrap();

        let names = registry.command_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"test1".to_string()));
        assert!(names.contains(&"test2".to_string()));
    }
}
