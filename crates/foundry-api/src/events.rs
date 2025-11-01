/// Event system for command lifecycle events
///
/// Provides events that are dispatched during the command execution lifecycle,
/// allowing applications to hook into command execution at various points.
///
/// # Events
///
/// - `CommandStarting` - Fired before a command executes
/// - `CommandFinished` - Fired after a command completes successfully
/// - `CommandFailed` - Fired when a command fails
///
/// # Example
///
/// ```rust,no_run
/// use foundry_api::events::{CommandEvent, EventDispatcher};
///
/// # async fn example() {
/// let dispatcher = EventDispatcher::new();
///
/// // Listen for command starting events
/// let dispatcher_clone = dispatcher.clone();
/// tokio::spawn(async move {
///     let mut rx = dispatcher_clone.subscribe();
///     while let Ok(event) = rx.recv().await {
///         match event {
///             CommandEvent::Starting(e) => {
///                 println!("Command starting: {}", e.command);
///             },
///             _ => {}
///         }
///     }
/// });
///
/// // Dispatch an event
/// dispatcher.dispatch(CommandEvent::starting("list", vec![])).await;
/// # }
/// ```

use foundry_plugins::{AppError, CommandResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error as log_error};

/// Maximum number of pending event messages in the broadcast channel
const EVENT_CHANNEL_CAPACITY: usize = 1000;

/// Command lifecycle events
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CommandEvent {
    /// Dispatched before a command starts executing
    Starting(CommandStartingEvent),
    /// Dispatched after a command finishes successfully
    Finished(CommandFinishedEvent),
    /// Dispatched when a command fails
    Failed(CommandFailedEvent),
}

impl CommandEvent {
    /// Create a CommandStarting event
    pub fn starting(command: impl Into<String>, args: Vec<String>) -> Self {
        CommandEvent::Starting(CommandStartingEvent {
            command: command.into(),
            args,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Create a CommandFinished event
    pub fn finished(
        command: impl Into<String>,
        result: CommandResult,
        duration_ms: u64,
    ) -> Self {
        CommandEvent::Finished(CommandFinishedEvent {
            command: command.into(),
            status: format!("{:?}", result.status),
            message: result.message,
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Create a CommandFailed event
    pub fn failed(
        command: impl Into<String>,
        error: AppError,
        duration_ms: u64,
    ) -> Self {
        CommandEvent::Failed(CommandFailedEvent {
            command: command.into(),
            error_code: error.code.clone(),
            error_message: error.message.clone(),
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

/// Event fired when a command starts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandStartingEvent {
    pub command: String,
    pub args: Vec<String>,
    pub timestamp: String,
}

/// Event fired when a command finishes successfully
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandFinishedEvent {
    pub command: String,
    pub status: String,
    pub message: Option<String>,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Event fired when a command fails
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandFailedEvent {
    pub command: String,
    pub error_code: String,
    pub error_message: String,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Event dispatcher for command lifecycle events
#[derive(Clone)]
pub struct EventDispatcher {
    tx: broadcast::Sender<CommandEvent>,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self { tx }
    }

    /// Subscribe to command events
    pub fn subscribe(&self) -> broadcast::Receiver<CommandEvent> {
        self.tx.subscribe()
    }

    /// Dispatch a command event
    pub async fn dispatch(&self, event: CommandEvent) {
        debug!("Dispatching event: {:?}", event);
        if let Err(e) = self.tx.send(event) {
            log_error!("Failed to dispatch event: {}", e);
        }
    }

    /// Dispatch a CommandStarting event
    pub async fn command_starting(&self, command: impl Into<String>, args: Vec<String>) {
        self.dispatch(CommandEvent::starting(command, args)).await;
    }

    /// Dispatch a CommandFinished event
    pub async fn command_finished(
        &self,
        command: impl Into<String>,
        result: CommandResult,
        duration_ms: u64,
    ) {
        self.dispatch(CommandEvent::finished(command, result, duration_ms))
            .await;
    }

    /// Dispatch a CommandFailed event
    pub async fn command_failed(
        &self,
        command: impl Into<String>,
        error: AppError,
        duration_ms: u64,
    ) {
        self.dispatch(CommandEvent::failed(command, error, duration_ms))
            .await;
    }

    /// Get number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Event listener trait for handling command events
#[async_trait::async_trait]
pub trait CommandEventListener: Send + Sync {
    /// Handle a command event
    async fn handle(&self, event: CommandEvent) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_event_creation() {
        let event = CommandEvent::starting("test", vec!["arg1".to_string()]);
        match event {
            CommandEvent::Starting(e) => {
                assert_eq!(e.command, "test");
                assert_eq!(e.args.len(), 1);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_event_dispatcher() {
        let dispatcher = EventDispatcher::new();
        let mut rx = dispatcher.subscribe();

        // Dispatch an event
        dispatcher
            .command_starting("test", vec!["arg1".to_string()])
            .await;

        // Receive the event
        let event = rx.recv().await.unwrap();
        match event {
            CommandEvent::Starting(e) => {
                assert_eq!(e.command, "test");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let dispatcher = EventDispatcher::new();

        let rx1 = dispatcher.subscribe();
        let rx2 = dispatcher.subscribe();

        assert_eq!(dispatcher.subscriber_count(), 2);

        dispatcher
            .command_starting("test", vec![])
            .await;

        // Both subscribers should receive the event
        let (mut rx1, mut rx2) = (rx1, rx2);
        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }
}
