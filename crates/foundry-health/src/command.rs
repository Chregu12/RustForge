//! Health check CLI command

use crate::{HealthCheckConfig, HealthChecker, HealthReport};
use anyhow::Result;
use async_trait::async_trait;
use foundry_plugins::{CommandContext, CommandResult, ResponseFormat, FoundryCommand, CommandError};
use foundry_domain::CommandDescriptor;

/// Health check command (health:check or doctor)
pub struct HealthCheckCommand;

#[async_trait]
impl FoundryCommand for HealthCheckCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        static DESCRIPTOR: once_cell::sync::Lazy<CommandDescriptor> = once_cell::sync::Lazy::new(|| {
            CommandDescriptor::builder("health:check", "health:check")
                .summary("Run comprehensive health checks on the application")
                .description("Performs system diagnostics including CPU, memory, disk space, and connectivity checks")
                .alias("doctor")
                .build()
        });
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Parse arguments for specific check
        let specific_check = ctx.args.first().map(|s| s.as_str());

        // Load environment variables
        let database_url = std::env::var("DATABASE_URL").ok();
        let cache_url = std::env::var("CACHE_URL").ok();

        let config = HealthCheckConfig {
            database_url,
            cache_url,
            ..Default::default()
        };

        let checker = HealthChecker::new(config);

        let results = if let Some(check_name) = specific_check {
            vec![checker.check_one(check_name).await
                .map_err(|e| CommandError::Message(e.to_string()))?]
        } else {
            checker.check_all().await
                .map_err(|e| CommandError::Message(e.to_string()))?
        };

        let report = HealthReport::new(results);

        // Format output
        let output = match ctx.format {
            ResponseFormat::Human => report.format_table(),
            ResponseFormat::Json => {
                serde_json::to_string_pretty(&report)
                    .map_err(|e| CommandError::Serialization(e.to_string()))?
            }
        };

        if report.all_passed() {
            Ok(CommandResult::success(output))
        } else {
            Ok(CommandResult::success(output).with_data(serde_json::json!({
                "overall_status": "failure",
                "checks": report.checks
            })))
        }
    }
}

/// Doctor command (alias for health:check)
pub struct DoctorCommand;

#[async_trait]
impl FoundryCommand for DoctorCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        static DESCRIPTOR: once_cell::sync::Lazy<CommandDescriptor> = once_cell::sync::Lazy::new(|| {
            CommandDescriptor::builder("doctor", "doctor")
                .summary("Run comprehensive health checks (alias for health:check)")
                .description("Performs system diagnostics including CPU, memory, disk space, and connectivity checks")
                .build()
        });
        &DESCRIPTOR
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        HealthCheckCommand.execute(ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::ExecutionOptions;
    use std::sync::Arc;

    // Create mock ports for testing
    struct MockArtifactPort;
    impl foundry_plugins::ArtifactPort for MockArtifactPort {
        fn write_file(&self, _path: &str, _contents: &str, _force: bool) -> Result<(), CommandError> {
            Ok(())
        }
    }

    struct MockMigrationPort;
    #[async_trait]
    impl foundry_plugins::MigrationPort for MockMigrationPort {
        async fn apply(&self, _config: &serde_json::Value, _dry_run: bool) -> Result<foundry_plugins::MigrationRun, CommandError> {
            Ok(foundry_plugins::MigrationRun::default())
        }
        async fn rollback(&self, _config: &serde_json::Value, _dry_run: bool) -> Result<foundry_plugins::MigrationRun, CommandError> {
            Ok(foundry_plugins::MigrationRun::default())
        }
    }

    struct MockSeedPort;
    #[async_trait]
    impl foundry_plugins::SeedPort for MockSeedPort {
        async fn run(&self, _config: &serde_json::Value, _dry_run: bool) -> Result<foundry_plugins::SeedRun, CommandError> {
            Ok(foundry_plugins::SeedRun::default())
        }
    }

    struct MockValidationPort;
    #[async_trait]
    impl foundry_plugins::ValidationPort for MockValidationPort {
        async fn validate(&self, _payload: serde_json::Value, _rules: foundry_plugins::ValidationRules) -> Result<foundry_plugins::ValidationReport, CommandError> {
            Ok(foundry_plugins::ValidationReport::valid())
        }
    }

    struct MockStoragePort;
    #[async_trait]
    impl foundry_plugins::StoragePort for MockStoragePort {
        async fn put(&self, _disk: &str, _path: &str, _contents: Vec<u8>) -> Result<foundry_plugins::StoredFile, CommandError> {
            Ok(foundry_plugins::StoredFile { disk: "local".to_string(), path: "test".to_string(), size: 0, url: None })
        }
        async fn get(&self, _disk: &str, _path: &str) -> Result<Vec<u8>, CommandError> {
            Ok(vec![])
        }
        async fn delete(&self, _disk: &str, _path: &str) -> Result<(), CommandError> {
            Ok(())
        }
        async fn exists(&self, _disk: &str, _path: &str) -> Result<bool, CommandError> {
            Ok(true)
        }
        async fn url(&self, _disk: &str, _path: &str) -> Result<String, CommandError> {
            Ok("http://localhost".to_string())
        }
    }

    struct MockCachePort;
    #[async_trait]
    impl foundry_plugins::CachePort for MockCachePort {
        async fn get(&self, _key: &str) -> Result<Option<serde_json::Value>, CommandError> {
            Ok(None)
        }
        async fn put(&self, _key: &str, _value: serde_json::Value, _ttl: Option<std::time::Duration>) -> Result<(), CommandError> {
            Ok(())
        }
        async fn forget(&self, _key: &str) -> Result<(), CommandError> {
            Ok(())
        }
        async fn clear(&self, _prefix: Option<&str>) -> Result<(), CommandError> {
            Ok(())
        }
    }

    struct MockQueuePort;
    #[async_trait]
    impl foundry_plugins::QueuePort for MockQueuePort {
        async fn dispatch(&self, _job: foundry_plugins::QueueJob) -> Result<(), CommandError> {
            Ok(())
        }
    }

    struct MockEventPort;
    #[async_trait]
    impl foundry_plugins::EventPort for MockEventPort {
        async fn publish(&self, _event: foundry_plugins::DomainEvent) -> Result<(), CommandError> {
            Ok(())
        }
    }

    fn create_test_context(args: Vec<String>) -> CommandContext {
        CommandContext {
            args,
            format: ResponseFormat::Human,
            metadata: serde_json::json!({}),
            config: serde_json::json!({}),
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
            artifacts: Arc::new(MockArtifactPort),
            migrations: Arc::new(MockMigrationPort),
            seeds: Arc::new(MockSeedPort),
            validation: Arc::new(MockValidationPort),
            storage: Arc::new(MockStoragePort),
            cache: Arc::new(MockCachePort),
            queue: Arc::new(MockQueuePort),
            events: Arc::new(MockEventPort),
        }
    }

    #[tokio::test]
    async fn test_health_check_command() {
        let cmd = HealthCheckCommand;
        let ctx = create_test_context(vec![]);
        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_health_check_specific() {
        let cmd = HealthCheckCommand;
        let ctx = create_test_context(vec!["rust".to_string()]);
        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_doctor_command() {
        let cmd = DoctorCommand;
        let ctx = create_test_context(vec![]);
        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.message.is_some());
    }
}
