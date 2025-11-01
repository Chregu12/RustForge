use foundry_application::TestCommand;
use foundry_infra::{
    FileStorageAdapter, InMemoryCacheStore, InMemoryEventBus, InMemoryQueue, LocalArtifactPort,
    SeaOrmMigrationService, SeaOrmSeedService, SimpleValidationService,
};
use foundry_plugins::{CommandContext, ExecutionOptions, FoundryCommand, ResponseFormat};
use foundry_storage::{config::StorageConfig, manager::StorageManager};
use serde_json::Value;
use std::sync::Arc;

fn create_test_context(args: Vec<String>, format: ResponseFormat) -> CommandContext {
    CommandContext {
        args,
        format,
        metadata: Value::Null,
        config: Value::Null,
        options: ExecutionOptions {
            dry_run: false,
            force: false,
        },
        artifacts: Arc::new(LocalArtifactPort::default()),
        migrations: Arc::new(SeaOrmMigrationService::default()),
        seeds: Arc::new(SeaOrmSeedService::default()),
        validation: Arc::new(SimpleValidationService::default()),
        storage: Arc::new(FileStorageAdapter::new(Arc::new(
            StorageManager::new(StorageConfig::from_env()).unwrap(),
        ))),
        cache: Arc::new(InMemoryCacheStore::default()),
        queue: Arc::new(InMemoryQueue::default()),
        events: Arc::new(InMemoryEventBus::default()),
    }
}

#[tokio::test]
async fn test_command_descriptor() {
    let command = TestCommand::new();
    let descriptor = command.descriptor();

    assert_eq!(descriptor.name, "test");
    assert!(!descriptor.summary.is_empty());
    assert!(descriptor.description.is_some());
}

#[tokio::test]
async fn test_command_executes_successfully() {
    let command = TestCommand::new();
    let ctx = create_test_context(vec![], ResponseFormat::Json);

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command execution should succeed");

    let cmd_result = result.unwrap();
    assert!(cmd_result.data.is_some(), "Result should contain data");

    let data = cmd_result.data.unwrap();
    assert!(
        data.get("statistics").is_some(),
        "Data should contain statistics"
    );
    assert!(
        data.get("exit_code").is_some(),
        "Data should contain exit_code"
    );
}

#[tokio::test]
async fn test_command_with_filter() {
    let command = TestCommand::new();
    let ctx = create_test_context(
        vec!["--filter=test_command".to_string()],
        ResponseFormat::Json,
    );

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command with filter should succeed");

    let cmd_result = result.unwrap();
    let data = cmd_result.data.unwrap();

    assert_eq!(
        data["filter"],
        "test_command",
        "Filter should be recorded in data"
    );
}

#[tokio::test]
async fn test_command_with_verbose() {
    let command = TestCommand::new();
    let ctx = create_test_context(vec!["--verbose".to_string()], ResponseFormat::Json);

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command with verbose should succeed");

    let cmd_result = result.unwrap();
    let data = cmd_result.data.unwrap();

    assert_eq!(
        data["verbose"], true,
        "Verbose flag should be recorded in data"
    );
}

#[tokio::test]
async fn test_command_with_coverage() {
    let command = TestCommand::new();
    let ctx = create_test_context(vec!["--coverage".to_string()], ResponseFormat::Json);

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command with coverage should succeed");

    let cmd_result = result.unwrap();
    let data = cmd_result.data.unwrap();

    assert_eq!(
        data["coverage"], true,
        "Coverage flag should be recorded in data"
    );
}

#[tokio::test]
async fn test_command_human_format() {
    let command = TestCommand::new();
    let ctx = create_test_context(vec![], ResponseFormat::Human);

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command with human format should succeed");

    let cmd_result = result.unwrap();
    assert!(
        cmd_result.message.is_some(),
        "Human format should include a message"
    );

    let message = cmd_result.message.unwrap();
    assert!(
        message.contains("Statistik") || message.contains("Tests"),
        "Message should contain test statistics or test info"
    );
}

#[tokio::test]
async fn test_command_json_format() {
    let command = TestCommand::new();
    let ctx = create_test_context(vec![], ResponseFormat::Json);

    let result = command.execute(ctx).await;
    assert!(result.is_ok(), "Command with JSON format should succeed");

    let cmd_result = result.unwrap();
    let data = cmd_result.data.unwrap();

    // Verify JSON structure
    assert!(data.is_object(), "Data should be a JSON object");
    assert!(data.get("statistics").is_some());
    assert!(data["statistics"].get("passed").is_some());
    assert!(data["statistics"].get("failed").is_some());
    assert!(data["statistics"].get("ignored").is_some());
    assert!(data["statistics"].get("total").is_some());
}

#[tokio::test]
async fn test_command_multiple_options() {
    let command = TestCommand::new();
    let ctx = create_test_context(
        vec![
            "--filter=integration".to_string(),
            "--verbose".to_string(),
            "--coverage".to_string(),
        ],
        ResponseFormat::Json,
    );

    let result = command.execute(ctx).await;
    assert!(
        result.is_ok(),
        "Command with multiple options should succeed"
    );

    let cmd_result = result.unwrap();
    let data = cmd_result.data.unwrap();

    assert_eq!(data["filter"], "integration");
    assert_eq!(data["verbose"], true);
    assert_eq!(data["coverage"], true);
}
