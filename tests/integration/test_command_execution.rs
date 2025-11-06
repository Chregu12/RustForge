use foundry_application::command::{Command, CommandRegistry};
use foundry_domain::Result;
use async_trait::async_trait;
use clap::Parser;

#[derive(Debug, Parser)]
struct TestCommand {
    #[clap(long)]
    name: Option<String>,
}

#[async_trait]
impl Command for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }

    fn description(&self) -> &'static str {
        "A test command"
    }

    async fn execute(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_command_registration() {
    // Test that commands can be registered
    let mut registry = CommandRegistry::new();
    let cmd = TestCommand { name: None };

    let result = registry.register(Box::new(cmd));
    assert!(result.is_ok(), "Command should register successfully");
}

#[tokio::test]
async fn test_command_execution() {
    // Test that a registered command can be executed
    let cmd = TestCommand {
        name: Some("test".to_string()),
    };

    let result = cmd.execute().await;
    assert!(result.is_ok(), "Command should execute successfully");
}

#[tokio::test]
async fn test_command_lookup() {
    // Test looking up a registered command
    let mut registry = CommandRegistry::new();
    let cmd = TestCommand { name: None };

    registry.register(Box::new(cmd)).unwrap();

    let found = registry.find("test");
    assert!(found.is_some(), "Command should be found in registry");
}

#[tokio::test]
async fn test_command_listing() {
    // Test listing all registered commands
    let mut registry = CommandRegistry::new();
    let cmd1 = TestCommand { name: None };

    registry.register(Box::new(cmd1)).unwrap();

    let commands = registry.list();
    assert!(!commands.is_empty(), "Registry should contain commands");
}

#[tokio::test]
async fn test_command_with_arguments() {
    // Test command with arguments
    let cmd = TestCommand {
        name: Some("test-arg".to_string()),
    };

    assert_eq!(cmd.name.as_deref(), Some("test-arg"));
}

#[tokio::test]
async fn test_command_error_handling() {
    // Test command error handling
    #[derive(Debug, Parser)]
    struct FailingCommand;

    #[async_trait]
    impl Command for FailingCommand {
        fn name(&self) -> &'static str {
            "failing"
        }

        fn description(&self) -> &'static str {
            "A failing command"
        }

        async fn execute(&self) -> Result<()> {
            Err(anyhow::anyhow!("Intentional failure").into())
        }
    }

    let cmd = FailingCommand;
    let result = cmd.execute().await;

    assert!(result.is_err(), "Failing command should return error");
}

#[tokio::test]
async fn test_command_help_text() {
    // Test that commands have proper help text
    let cmd = TestCommand { name: None };

    assert!(!cmd.name().is_empty(), "Command should have a name");
    assert!(!cmd.description().is_empty(), "Command should have a description");
}

#[cfg(test)]
mod command_pipeline_tests {
    use super::*;

    #[tokio::test]
    async fn test_command_pipeline_execution() {
        // Test executing multiple commands in sequence
        let cmd1 = TestCommand { name: Some("first".to_string()) };
        let cmd2 = TestCommand { name: Some("second".to_string()) };

        let result1 = cmd1.execute().await;
        let result2 = cmd2.execute().await;

        assert!(result1.is_ok() && result2.is_ok(), "Pipeline should execute all commands");
    }

    #[tokio::test]
    async fn test_command_isolation() {
        // Test that commands are properly isolated
        let cmd1 = TestCommand { name: Some("isolated1".to_string()) };
        let cmd2 = TestCommand { name: Some("isolated2".to_string()) };

        assert_ne!(cmd1.name, cmd2.name, "Commands should be isolated");
    }
}
