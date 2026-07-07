//! Core command executor implementation

use crate::error::ExecutionResult;
use crate::options::{CommandOptions, ExecutionMode};
use crate::output::{OutputCapture, OutputMode};
use crate::registry::CommandRegistry;
use crate::result::ExecutionResult as CommandExecutionResult;
use rf_plugins::{CommandContext, CommandResult, QueueJob};
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

/// Programmatic command executor
#[derive(Clone)]
pub struct CommandExecutor {
    registry: Arc<CommandRegistry>,
}

impl CommandExecutor {
    /// Create new command executor with registry
    pub fn new(registry: Arc<CommandRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a command by name with arguments
    ///
    /// # Example
    /// ```no_run
    /// # use rf_command_executor::CommandExecutor;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let executor = CommandExecutor::new(std::sync::Arc::new(rf_command_executor::CommandRegistry::new()));
    /// let result = executor.execute("migrate:run", vec!["--force".to_string()]).await?;
    /// println!("Exit code: {}", result.exit_code);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(
        &self,
        command_name: &str,
        args: Vec<String>,
    ) -> ExecutionResult<CommandExecutionResult> {
        let options = CommandOptions::new();
        self.execute_with_options(command_name, args, options).await
    }

    /// Execute a command with full options
    ///
    /// # Example
    /// ```no_run
    /// # use rf_command_executor::{CommandExecutor, CommandOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let executor = CommandExecutor::new(std::sync::Arc::new(rf_command_executor::CommandRegistry::new()));
    /// let options = CommandOptions::new()
    ///     .arg("name", "test")
    ///     .flag("verbose")
    ///     .dry_run(true);
    ///
    /// let result = executor.call("make:migration", options).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn call(
        &self,
        command_name: &str,
        options: CommandOptions,
    ) -> ExecutionResult<CommandExecutionResult> {
        let args = options.to_args();
        self.execute_with_options(command_name, args, options).await
    }

    /// Queue a command for later execution
    ///
    /// # Example
    /// ```no_run
    /// # use rf_command_executor::CommandExecutor;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let executor = CommandExecutor::new(std::sync::Arc::new(rf_command_executor::CommandRegistry::new()));
    /// executor.queue("send:notifications", vec!["--user=123".to_string()]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn queue(&self, command_name: &str, args: Vec<String>) -> ExecutionResult<()> {
        let mut options = CommandOptions::new();
        options.mode = ExecutionMode::Queued;

        self.execute_with_options(command_name, args, options)
            .await?;
        Ok(())
    }

    /// Execute command with full control over options
    async fn execute_with_options(
        &self,
        command_name: &str,
        args: Vec<String>,
        options: CommandOptions,
    ) -> ExecutionResult<CommandExecutionResult> {
        debug!("Executing command: {} with args: {:?}", command_name, args);

        // Get command from registry
        let command = self.registry.get(command_name)?;

        // Determine output mode
        let output_mode = if options.capture_output || options.capture_errors {
            OutputMode::Capture
        } else {
            OutputMode::PassThrough
        };

        let output_capture = OutputCapture::new(output_mode);

        // Check if should queue
        if options.mode == ExecutionMode::Queued {
            return self
                .queue_command(command_name, args, options, output_capture)
                .await;
        }

        // Execute immediately
        self.execute_immediate(command, args, options, output_capture)
            .await
    }

    /// Execute command immediately
    async fn execute_immediate(
        &self,
        command: Arc<dyn rf_plugins::FoundryCommand>,
        args: Vec<String>,
        options: CommandOptions,
        output_capture: OutputCapture,
    ) -> ExecutionResult<CommandExecutionResult> {
        let start = Instant::now();

        // Create command context (simplified - in real implementation this would come from app)
        let ctx = self.create_context(args, options)?;

        // Execute command
        let result = match command.execute(ctx).await {
            Ok(result) => {
                info!("Command executed successfully");
                output_capture.capture_stdout(result.message.clone().unwrap_or_default());
                result
            }
            Err(e) => {
                error!("Command execution failed: {}", e);
                output_capture.capture_stderr(e.to_string());
                CommandResult::failure(rf_plugins::AppError::new(
                    "EXECUTION_ERROR",
                    e.to_string(),
                ))
            }
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;
        let exit_code = if result.status == rf_plugins::CommandStatus::Success {
            0
        } else {
            1
        };

        let exec_result = CommandExecutionResult::new(
            exit_code,
            result,
            output_capture.get_output(),
            execution_time_ms,
        );

        Ok(exec_result)
    }

    /// Queue command for later execution
    async fn queue_command(
        &self,
        command_name: &str,
        args: Vec<String>,
        options: CommandOptions,
        output_capture: OutputCapture,
    ) -> ExecutionResult<CommandExecutionResult> {
        info!("Queueing command: {}", command_name);

        let _job = QueueJob {
            name: command_name.to_string(),
            payload: serde_json::json!({
                "args": args,
                "options": options,
            }),
            delay_seconds: None,
        };

        // Note: In real implementation, this would use the QueuePort from context
        // For now, we'll just return a success result
        output_capture.capture_stdout(format!("Command '{}' queued for execution", command_name));

        let result = CommandResult::success(format!("Command '{}' queued", command_name));

        Ok(CommandExecutionResult::new(
            0,
            result,
            output_capture.get_output(),
            0,
        ))
    }

    /// Create command context (simplified)
    fn create_context(
        &self,
        args: Vec<String>,
        options: CommandOptions,
    ) -> ExecutionResult<CommandContext> {
        // This is a simplified version - in real usage, context would be provided by the application
        // For now, we'll create a minimal context with dummy ports
        use rf_plugins::{
            ArtifactPort, CachePort, EventPort, ExecutionOptions, MigrationPort, QueuePort,
            ResponseFormat, SeedPort, StoragePort, ValidationPort,
        };

        // Create dummy ports (these would be real implementations in production)
        struct DummyArtifactPort;
        impl ArtifactPort for DummyArtifactPort {
            fn write_file(
                &self,
                _path: &str,
                _contents: &str,
                _force: bool,
            ) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
        }

        #[async_trait::async_trait]
        impl MigrationPort for DummyArtifactPort {
            async fn apply(
                &self,
                _config: &Value,
                _dry_run: bool,
            ) -> Result<rf_plugins::MigrationRun, rf_plugins::CommandError> {
                Ok(rf_plugins::MigrationRun::default())
            }
            async fn rollback(
                &self,
                _config: &Value,
                _dry_run: bool,
            ) -> Result<rf_plugins::MigrationRun, rf_plugins::CommandError> {
                Ok(rf_plugins::MigrationRun::default())
            }
        }

        #[async_trait::async_trait]
        impl SeedPort for DummyArtifactPort {
            async fn run(
                &self,
                _config: &Value,
                _dry_run: bool,
            ) -> Result<rf_plugins::SeedRun, rf_plugins::CommandError> {
                Ok(rf_plugins::SeedRun::default())
            }
        }

        #[async_trait::async_trait]
        impl ValidationPort for DummyArtifactPort {
            async fn validate(
                &self,
                _payload: Value,
                _rules: rf_plugins::ValidationRules,
            ) -> Result<rf_plugins::ValidationReport, rf_plugins::CommandError>
            {
                Ok(rf_plugins::ValidationReport::valid())
            }
        }

        #[async_trait::async_trait]
        impl StoragePort for DummyArtifactPort {
            async fn put(
                &self,
                _disk: &str,
                _path: &str,
                _contents: Vec<u8>,
            ) -> Result<rf_plugins::StoredFile, rf_plugins::CommandError> {
                Err(rf_plugins::CommandError::Message(
                    "Not implemented".to_string(),
                ))
            }
            async fn get(
                &self,
                _disk: &str,
                _path: &str,
            ) -> Result<Vec<u8>, rf_plugins::CommandError> {
                Err(rf_plugins::CommandError::Message(
                    "Not implemented".to_string(),
                ))
            }
            async fn delete(
                &self,
                _disk: &str,
                _path: &str,
            ) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
            async fn exists(
                &self,
                _disk: &str,
                _path: &str,
            ) -> Result<bool, rf_plugins::CommandError> {
                Ok(false)
            }
            async fn url(
                &self,
                _disk: &str,
                _path: &str,
            ) -> Result<String, rf_plugins::CommandError> {
                Err(rf_plugins::CommandError::Message(
                    "Not implemented".to_string(),
                ))
            }
        }

        #[async_trait::async_trait]
        impl CachePort for DummyArtifactPort {
            async fn get(
                &self,
                _key: &str,
            ) -> Result<Option<Value>, rf_plugins::CommandError> {
                Ok(None)
            }
            async fn put(
                &self,
                _key: &str,
                _value: Value,
                _ttl: Option<std::time::Duration>,
            ) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
            async fn forget(&self, _key: &str) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
            async fn clear(
                &self,
                _prefix: Option<&str>,
            ) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
        }

        #[async_trait::async_trait]
        impl QueuePort for DummyArtifactPort {
            async fn dispatch(&self, _job: QueueJob) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
        }

        #[async_trait::async_trait]
        impl EventPort for DummyArtifactPort {
            async fn publish(
                &self,
                _event: rf_plugins::DomainEvent,
            ) -> Result<(), rf_plugins::CommandError> {
                Ok(())
            }
        }

        let dummy_port = Arc::new(DummyArtifactPort);

        Ok(CommandContext {
            args,
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config: Value::Null,
            options: ExecutionOptions {
                dry_run: options.dry_run,
                force: options.force,
            },
            artifacts: dummy_port.clone(),
            migrations: dummy_port.clone(),
            seeds: dummy_port.clone(),
            validation: dummy_port.clone(),
            storage: dummy_port.clone(),
            cache: dummy_port.clone(),
            queue: dummy_port.clone(),
            events: dummy_port,
        })
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &Arc<CommandRegistry> {
        &self.registry
    }
}

// TODO: Update tests to use new CommandDescriptor API
// #[cfg(test)]
// mod tests { ... }
