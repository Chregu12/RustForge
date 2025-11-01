use crate::invocation::{CommandInvoker, FoundryInvoker, InvocationRequest};
use foundry_application::ApplicationError;
use foundry_plugins::{CommandResult, ResponseFormat};
use serde_json::json;
use std::sync::{Arc, Mutex};

/// Artisan - Laravel-like facade for programmatic command execution
///
/// Provides a simple, intuitive API for calling commands from Rust code,
/// similar to Laravel's `Artisan::call()` method.
///
/// # Examples
///
/// ```rust,no_run
/// # use foundry_api::Artisan;
/// # use foundry_api::FoundryInvoker;
/// # use foundry_application::FoundryApp;
/// #
/// # async fn example(invoker: FoundryInvoker) -> anyhow::Result<()> {
/// let artisan = Artisan::new(invoker);
///
/// // Simple command execution
/// let result = artisan.call("migrate").await?;
/// println!("Output: {}", result.message().unwrap_or_default());
///
/// // With arguments
/// let result = artisan
///     .call("make:command")
///     .with_args(vec!["TestCommand".to_string()])
///     .dispatch()
///     .await?;
///
/// // Command chaining
/// let results = artisan
///     .chain()
///     .add("migrate")
///     .add_with_args("seed:run", vec!["--class".to_string(), "DatabaseSeeder".to_string()])
///     .dispatch()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Artisan {
    invoker: FoundryInvoker,
    captured_output: Arc<Mutex<Vec<String>>>,
}

impl Artisan {
    /// Create a new Artisan instance
    pub fn new(invoker: FoundryInvoker) -> Self {
        Self {
            invoker,
            captured_output: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Create a command execution builder for a specific command
    pub fn call(&self, command: impl Into<String>) -> CommandBuilder {
        CommandBuilder::new(
            self.invoker.clone(),
            command.into(),
            self.captured_output.clone(),
        )
    }

    /// Create a command chain builder
    pub fn chain(&self) -> CommandChain {
        CommandChain::new(self.invoker.clone(), self.captured_output.clone())
    }

    /// Get the captured output from the last command
    pub fn output(&self) -> Vec<String> {
        self.captured_output.lock().unwrap().clone()
    }

    /// Get the captured output as a single string
    pub fn output_string(&self) -> String {
        self.output().join("\n")
    }

    /// Clear the captured output
    pub fn clear_output(&self) {
        self.captured_output.lock().unwrap().clear();
    }
}

/// Builder for a single command execution
pub struct CommandBuilder {
    invoker: FoundryInvoker,
    command: String,
    args: Vec<String>,
    captured_output: Arc<Mutex<Vec<String>>>,
    format: ResponseFormat,
    dry_run: bool,
    force: bool,
}

impl CommandBuilder {
    pub fn new(
        invoker: FoundryInvoker,
        command: String,
        captured_output: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        Self {
            invoker,
            command,
            args: Vec::new(),
            captured_output,
            format: ResponseFormat::Json,
            dry_run: false,
            force: false,
        }
    }

    /// Add arguments to the command
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Add a single argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Set the output format
    pub fn with_format(mut self, format: ResponseFormat) -> Self {
        self.format = format;
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

    /// Execute the command synchronously (blocking)
    pub fn dispatch_blocking(self) -> Result<CommandResult, ApplicationError> {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(self.dispatch())
    }

    /// Execute the command asynchronously
    pub async fn dispatch(self) -> Result<CommandResult, ApplicationError> {
        let options = foundry_plugins::ExecutionOptions {
            dry_run: self.dry_run,
            force: self.force,
        };

        let request = InvocationRequest::new(self.command)
            .with_args(self.args)
            .with_format(self.format)
            .with_options(options);

        let result = self.invoker.invoke(request).await?;

        // Capture output
        if let Some(message) = &result.message {
            self.captured_output.lock().unwrap().push(message.clone());
        }

        Ok(result)
    }
}

/// Builder for chaining multiple commands
pub struct CommandChain {
    invoker: FoundryInvoker,
    commands: Vec<(String, Vec<String>)>,
    captured_output: Arc<Mutex<Vec<String>>>,
    stop_on_error: bool,
}

impl CommandChain {
    pub fn new(invoker: FoundryInvoker, captured_output: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            invoker,
            commands: Vec::new(),
            captured_output,
            stop_on_error: true,
        }
    }

    /// Add a command to the chain
    pub fn add(mut self, command: impl Into<String>) -> Self {
        self.commands.push((command.into(), Vec::new()));
        self
    }

    /// Add a command with arguments to the chain
    pub fn add_with_args(mut self, command: impl Into<String>, args: Vec<String>) -> Self {
        self.commands.push((command.into(), args));
        self
    }

    /// Whether to stop on first error (default: true)
    pub fn stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// Execute all commands in the chain
    pub async fn dispatch(
        self,
    ) -> Result<Vec<CommandResult>, (usize, ApplicationError, Vec<CommandResult>)> {
        let mut results = Vec::new();

        for (idx, (command, args)) in self.commands.iter().enumerate() {
            let options = foundry_plugins::ExecutionOptions::default();
            let request = InvocationRequest::new(command.clone())
                .with_args(args.clone())
                .with_format(ResponseFormat::Json)
                .with_options(options);

            match self.invoker.invoke(request).await {
                Ok(result) => {
                    if let Some(message) = &result.message {
                        self.captured_output.lock().unwrap().push(message.clone());
                    }
                    results.push(result);
                }
                Err(err) => {
                    if self.stop_on_error {
                        return Err((idx, err, results));
                    }
                    // Continue on error if stop_on_error is false
                }
            }
        }

        Ok(results)
    }

    /// Execute all commands in the chain synchronously (blocking)
    pub fn dispatch_blocking(
        self,
    ) -> Result<Vec<CommandResult>, (usize, ApplicationError, Vec<CommandResult>)> {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(self.dispatch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder_creation() {
        // This is a basic test to ensure types are correct
        // Full integration tests require a running FoundryApp
        let captured = Arc::new(Mutex::new(Vec::new()));

        let builder = CommandBuilder {
            invoker: {
                // Mock invoker - in real tests, use actual instance
                panic!("Use integration tests instead")
            },
            command: "test".to_string(),
            args: vec!["arg1".to_string()],
            captured_output: captured,
            format: ResponseFormat::Json,
            dry_run: false,
            force: false,
        };

        assert_eq!(builder.command, "test");
        assert_eq!(builder.args.len(), 1);
    }
}
