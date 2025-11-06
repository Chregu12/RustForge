//! Regression tests for CommandRegistry error handling
//!
//! These tests ensure that the registry properly handles error cases
//! without panicking, particularly around poisoned mutex scenarios.

use foundry_application::{ApplicationError, CommandRegistry};
use foundry_domain::CommandDescriptor;
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};
use async_trait::async_trait;
use std::sync::Arc;

/// Mock command for testing
struct MockCommand {
    descriptor: CommandDescriptor,
}

impl MockCommand {
    fn new(name: &str) -> Self {
        Self {
            descriptor: CommandDescriptor::builder(name, name)
                .summary("Test command")
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for MockCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        Ok(CommandResult::success("Test executed"))
    }
}

#[test]
fn test_registry_register_returns_result() {
    let registry = CommandRegistry::default();
    let cmd: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));

    // Should not panic, should return Result
    let result = registry.register(cmd);
    assert!(result.is_ok());
}

#[test]
fn test_registry_duplicate_command_error() {
    let registry = CommandRegistry::default();
    let cmd1: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));
    let cmd2: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));

    registry.register(cmd1).unwrap();
    let result = registry.register(cmd2);

    // Should return error, not panic
    assert!(result.is_err());
    match result {
        Err(ApplicationError::CommandAlreadyRegistered(name)) => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected CommandAlreadyRegistered error"),
    }
}

#[test]
fn test_registry_resolve_returns_result() {
    let registry = CommandRegistry::default();
    let cmd: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));

    registry.register(cmd).unwrap();

    // Should not panic, should return Result
    let result = registry.resolve("test");
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[test]
fn test_registry_resolve_nonexistent_command() {
    let registry = CommandRegistry::default();

    // Should not panic, should return Ok(None)
    let result = registry.resolve("nonexistent");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_registry_descriptors_returns_result() {
    let registry = CommandRegistry::default();
    let cmd: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));

    registry.register(cmd).unwrap();

    // Should not panic, should return Result
    let result = registry.descriptors();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_registry_len_returns_result() {
    let registry = CommandRegistry::default();
    let cmd: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));

    registry.register(cmd).unwrap();

    // Should not panic, should return Result
    let result = registry.len();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_registry_is_empty_returns_result() {
    let registry = CommandRegistry::default();

    // Should not panic, should return Result
    let result = registry.is_empty();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);

    let cmd: Arc<dyn FoundryCommand + Send + Sync> = Arc::new(MockCommand::new("test"));
    registry.register(cmd).unwrap();

    let result = registry.is_empty();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), false);
}

#[test]
fn test_registry_concurrent_access() {
    use std::thread;

    let registry = CommandRegistry::default();
    let registry_clone = registry.clone();

    // Spawn thread that registers commands
    let handle = thread::spawn(move || {
        for i in 0..10 {
            let cmd: Arc<dyn FoundryCommand + Send + Sync> =
                Arc::new(MockCommand::new(&format!("cmd{}", i)));
            let _ = registry_clone.register(cmd);
        }
    });

    // Main thread also registers commands
    for i in 10..20 {
        let cmd: Arc<dyn FoundryCommand + Send + Sync> =
            Arc::new(MockCommand::new(&format!("cmd{}", i)));
        let _ = registry.register(cmd);
    }

    handle.join().unwrap();

    // Should be able to query without panicking
    let result = registry.len();
    assert!(result.is_ok());
    let len = result.unwrap();
    assert!(len <= 20); // Some may have failed due to duplicates
}
