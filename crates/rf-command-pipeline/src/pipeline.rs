use crate::error::{PipelineError, PipelineResult};
use crate::executor::CommandExecutor;
use crate::result::{CommandResult, ExecutionSummary};
use std::time::Instant;

/// A command to be executed in the pipeline
#[derive(Debug, Clone)]
struct PipelineCommand {
    name: String,
    args: Vec<String>,
    condition: CommandCondition,
}

/// Condition for command execution
#[derive(Debug, Clone)]
enum CommandCondition {
    /// Always execute
    Always,
    /// Execute only if previous command succeeded
    OnSuccess,
    /// Execute only if previous command failed
    OnFailure,
}

/// Builder for creating pipelines
pub struct PipelineBuilder<E: CommandExecutor> {
    executor: E,
    commands: Vec<PipelineCommand>,
    stop_on_error: bool,
}

impl<E: CommandExecutor> PipelineBuilder<E> {
    /// Create a new pipeline builder
    pub fn new(executor: E) -> Self {
        Self {
            executor,
            commands: Vec::new(),
            stop_on_error: true,
        }
    }

    /// Add a command to execute
    pub fn then(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.commands.push(PipelineCommand {
            name: command.into(),
            args,
            condition: CommandCondition::Always,
        });
        self
    }

    /// Add a command that only executes if the previous command succeeded
    pub fn on_success(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.commands.push(PipelineCommand {
            name: command.into(),
            args,
            condition: CommandCondition::OnSuccess,
        });
        self
    }

    /// Add a command that only executes if the previous command failed
    pub fn on_failure(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.commands.push(PipelineCommand {
            name: command.into(),
            args,
            condition: CommandCondition::OnFailure,
        });
        self
    }

    /// Set whether to stop on error (default: true)
    pub fn stop_on_error(&mut self, stop: bool) -> &mut Self {
        self.stop_on_error = stop;
        self
    }

    /// Execute the pipeline sequentially
    pub async fn execute(&self) -> PipelineResult<ExecutionSummary> {
        let mut summary = ExecutionSummary::new();
        let mut last_success = true;

        for cmd in &self.commands {
            // Check condition
            let should_execute = match cmd.condition {
                CommandCondition::Always => true,
                CommandCondition::OnSuccess => last_success,
                CommandCondition::OnFailure => !last_success,
            };

            if !should_execute {
                summary.add_skipped();
                continue;
            }

            // Execute command
            let start = Instant::now();
            match self.executor.execute(&cmd.name, cmd.args.clone()).await {
                Ok(output) => {
                    let duration = start.elapsed();
                    let result = CommandResult::success(cmd.name.clone(), cmd.args.clone(), output, duration);
                    summary.add_result(result);
                    last_success = true;
                }
                Err(error) => {
                    let duration = start.elapsed();
                    let result = CommandResult::failure(cmd.name.clone(), cmd.args.clone(), error.clone(), duration);
                    summary.add_result(result);
                    last_success = false;

                    if self.stop_on_error {
                        return Err(PipelineError::stopped(&cmd.name));
                    }
                }
            }
        }

        if summary.failed > 0 && self.stop_on_error {
            Err(PipelineError::multiple_failed(
                summary
                    .failed_commands()
                    .iter()
                    .map(|r| r.error.clone().unwrap_or_default())
                    .collect(),
            ))
        } else {
            Ok(summary)
        }
    }

    /// Execute multiple commands in parallel
    pub async fn execute_parallel(&self, commands: Vec<(&str, Vec<String>)>) -> PipelineResult<ExecutionSummary> {
        let mut handles = Vec::new();

        for (command, args) in commands {
            let executor = self.executor.clone();
            let cmd = command.to_string();
            let args_clone = args.clone();

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                match executor.execute(&cmd, args_clone.clone()).await {
                    Ok(output) => {
                        let duration = start.elapsed();
                        CommandResult::success(cmd, args_clone, output, duration)
                    }
                    Err(error) => {
                        let duration = start.elapsed();
                        CommandResult::failure(cmd, args_clone, error, duration)
                    }
                }
            });

            handles.push(handle);
        }

        let mut summary = ExecutionSummary::new();

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    summary.add_result(result);
                }
                Err(e) => {
                    return Err(PipelineError::custom(format!("Task join error: {}", e)));
                }
            }
        }

        if summary.failed > 0 {
            Err(PipelineError::multiple_failed(
                summary
                    .failed_commands()
                    .iter()
                    .map(|r| r.error.clone().unwrap_or_default())
                    .collect(),
            ))
        } else {
            Ok(summary)
        }
    }
}

/// Pipeline for command execution
pub struct Pipeline<E: CommandExecutor> {
    builder: PipelineBuilder<E>,
}

impl<E: CommandExecutor> Pipeline<E> {
    /// Create a new pipeline
    pub fn new(executor: E) -> Self {
        Self {
            builder: PipelineBuilder::new(executor),
        }
    }

    /// Add a command to execute
    pub fn then(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.builder.then(command, args);
        self
    }

    /// Add a command that only executes if the previous command succeeded
    pub fn on_success(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.builder.on_success(command, args);
        self
    }

    /// Add a command that only executes if the previous command failed
    pub fn on_failure(&mut self, command: impl Into<String>, args: Vec<String>) -> &mut Self {
        self.builder.on_failure(command, args);
        self
    }

    /// Set whether to stop on error (default: true)
    pub fn stop_on_error(&mut self, stop: bool) -> &mut Self {
        self.builder.stop_on_error(stop);
        self
    }

    /// Execute the pipeline
    pub async fn execute(&self) -> PipelineResult<ExecutionSummary> {
        self.builder.execute().await
    }

    /// Execute commands in parallel
    pub async fn parallel(&self, commands: Vec<(&str, Vec<String>)>) -> PipelineResult<ExecutionSummary> {
        self.builder.execute_parallel(commands).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::DummyExecutor;

    #[tokio::test]
    async fn test_pipeline_sequential() {
        let executor = DummyExecutor;
        let mut pipeline = Pipeline::new(executor);

        pipeline
            .then("cmd1", vec![])
            .then("cmd2", vec![])
            .then("cmd3", vec![]);

        let result = pipeline.execute().await;
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 3);
    }

    #[tokio::test]
    async fn test_pipeline_conditional() {
        let executor = DummyExecutor;
        let mut pipeline = Pipeline::new(executor);

        pipeline
            .then("cmd1", vec![])
            .on_success("success_cmd", vec![])
            .on_failure("failure_cmd", vec![]);

        let result = pipeline.execute().await;
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.succeeded, 2); // cmd1 and success_cmd
        assert_eq!(summary.skipped, 1); // failure_cmd
    }

    #[tokio::test]
    async fn test_pipeline_parallel() {
        let executor = DummyExecutor;
        let pipeline = Pipeline::new(executor);

        let commands = vec![
            ("cmd1", vec![]),
            ("cmd2", vec![]),
            ("cmd3", vec![]),
        ];

        let result = pipeline.parallel(commands).await;
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.succeeded, 3);
    }

    #[derive(Clone)]
    struct FailingExecutor;

    #[async_trait::async_trait]
    impl CommandExecutor for FailingExecutor {
        async fn execute(&self, command: &str, _args: Vec<String>) -> Result<String, String> {
            if command == "fail" {
                Err("Command failed".to_string())
            } else {
                Ok(format!("Executed: {}", command))
            }
        }
    }

    #[tokio::test]
    async fn test_pipeline_stop_on_error() {
        let executor = FailingExecutor;
        let mut pipeline = Pipeline::new(executor);

        pipeline
            .then("ok1", vec![])
            .then("fail", vec![])
            .then("ok2", vec![]);

        let result = pipeline.execute().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_continue_on_error() {
        let executor = FailingExecutor;
        let mut pipeline = Pipeline::new(executor);

        pipeline
            .stop_on_error(false)
            .then("ok1", vec![])
            .then("fail", vec![])
            .then("ok2", vec![]);

        let result = pipeline.execute().await;
        assert!(result.is_err()); // Still returns error, but executes all commands

        // We would need to access the summary to verify all commands ran
        // In a real implementation, you might want to return Result<Summary, Vec<Error>>
    }
}
