/// Event-dispatching command invoker
///
/// Wraps a regular invoker and automatically dispatches command lifecycle events.
/// This allows applications to hook into command execution without modifying commands.

use crate::events::{CommandEvent, EventDispatcher};
use crate::invocation::{CommandInvoker, InvocationRequest};
use foundry_application::ApplicationError;
use foundry_plugins::CommandResult;
use std::time::Instant;
use tracing::{debug, info};

/// A command invoker that dispatches events during command execution
#[derive(Clone)]
pub struct EventDispatchingInvoker {
    inner: Box<dyn CommandInvoker>,
    dispatcher: EventDispatcher,
}

impl EventDispatchingInvoker {
    /// Create a new event-dispatching invoker
    ///
    /// # Arguments
    ///
    /// * `inner` - The underlying command invoker to delegate to
    /// * `dispatcher` - The event dispatcher to publish events to
    pub fn new(inner: Box<dyn CommandInvoker>, dispatcher: EventDispatcher) -> Self {
        Self { inner, dispatcher }
    }

    /// Get a reference to the event dispatcher
    pub fn dispatcher(&self) -> &EventDispatcher {
        &self.dispatcher
    }
}

#[async_trait::async_trait]
impl CommandInvoker for EventDispatchingInvoker {
    async fn invoke(&self, request: InvocationRequest) -> Result<CommandResult, ApplicationError> {
        let start = Instant::now();
        let command = request.command.clone();
        let args = request.args.clone();

        debug!("Dispatching CommandStarting event for command: {}", command);

        // Dispatch CommandStarting event
        self.dispatcher
            .command_starting(command.clone(), args.clone())
            .await;

        // Execute the command
        match self.inner.invoke(request).await {
            Ok(result) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let status = format!("{:?}", result.status);

                info!(
                    "Command completed: {} (status: {}, duration: {}ms)",
                    command, status, duration_ms
                );

                // Dispatch CommandFinished event
                self.dispatcher
                    .command_finished(command.clone(), result.clone(), duration_ms)
                    .await;

                Ok(result)
            }
            Err(err) => {
                let duration_ms = start.elapsed().as_millis() as u64;

                // Convert ApplicationError to AppError for event
                let app_error = match err.clone() {
                    ApplicationError::CommandNotFound(cmd) => foundry_plugins::AppError::new(
                        "COMMAND_NOT_FOUND",
                        format!("Command `{}` not found", cmd),
                    ),
                    ApplicationError::CommandAlreadyRegistered(cmd) => {
                        foundry_plugins::AppError::new(
                            "COMMAND_ALREADY_REGISTERED",
                            format!("Command `{}` already registered", cmd),
                        )
                    }
                    ApplicationError::CommandExecution(msg) => foundry_plugins::AppError::new(
                        "COMMAND_EXECUTION_ERROR",
                        format!("Command execution failed: {}", msg),
                    ),
                    ApplicationError::StorageError(msg) => {
                        foundry_plugins::AppError::new("STORAGE_ERROR", msg)
                    }
                };

                info!(
                    "Command failed: {} (error: {}, duration: {}ms)",
                    command,
                    app_error.code,
                    duration_ms
                );

                // Dispatch CommandFailed event
                self.dispatcher
                    .command_failed(command.clone(), app_error, duration_ms)
                    .await;

                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_dispatching_invoker_creation() {
        // This would require a mock invoker implementation
        // For now, just test that the struct can be created
        let _dispatcher = EventDispatcher::new();
        // In real tests, we would create a mock invoker
    }
}
