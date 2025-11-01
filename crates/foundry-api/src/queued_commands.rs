/// Queued Commands System
///
/// Allows commands to be dispatched to a job queue for asynchronous execution,
/// similar to Laravel's queue:dispatch() for commands.
///
/// This is useful for long-running operations that shouldn't block the user:
/// - Email sending
/// - File processing
/// - Report generation
/// - Data synchronization
///
/// # Example
///
/// ```rust,no_run
/// use foundry_api::queued_commands::{QueuedCommand, CommandQueue};
///
/// let queued = QueuedCommand::new("import:data")
///     .with_args(vec!["file.csv".to_string()])
///     .with_delay(std::time::Duration::from_secs(60));
///
/// let queue = CommandQueue::default();
/// queue.dispatch(queued).await?;
/// ```

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A command queued for asynchronous execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueuedCommand {
    /// Command name
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Delay before execution (in seconds)
    pub delay: Option<Duration>,
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Timeout for execution
    pub timeout: Option<Duration>,
    /// Queue name
    pub queue: String,
    /// Unique job ID
    pub job_id: String,
    /// Custom metadata
    pub metadata: serde_json::Value,
}

impl QueuedCommand {
    /// Create a new queued command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            delay: None,
            max_attempts: 3,
            timeout: None,
            queue: "default".to_string(),
            job_id: uuid::Uuid::new_v4().to_string(),
            metadata: serde_json::json!({}),
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

    /// Delay execution by the specified duration
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Set the maximum number of retry attempts
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    /// Set the execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the queue name
    pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = queue.into();
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut obj) = self.metadata {
            obj.insert(key.into(), value);
        }
        self
    }

    /// Get job ID
    pub fn id(&self) -> &str {
        &self.job_id
    }

    /// Get delay in seconds
    pub fn delay_seconds(&self) -> u64 {
        self.delay
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Check if job should be delayed
    pub fn is_delayed(&self) -> bool {
        self.delay.is_some()
    }
}

/// Command queue for managing queued commands
#[derive(Clone)]
pub struct CommandQueue {
    name: String,
}

impl CommandQueue {
    /// Create a new command queue
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }

    /// Get the queue name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Dispatch a command to the queue
    pub async fn dispatch(&self, command: QueuedCommand) -> Result<String, QueueError> {
        // In a real implementation, this would interact with the actual queue system
        // For now, we return the job ID as a success indicator

        if command.command.is_empty() {
            return Err(QueueError::InvalidCommand("Command name is empty".to_string()));
        }

        if command.max_attempts == 0 {
            return Err(QueueError::InvalidCommand(
                "max_attempts must be at least 1".to_string(),
            ));
        }

        Ok(command.job_id)
    }

    /// Dispatch multiple commands
    pub async fn dispatch_many(
        &self,
        commands: Vec<QueuedCommand>,
    ) -> Result<Vec<String>, QueueError> {
        let mut ids = Vec::new();

        for command in commands {
            let id = self.dispatch(command).await?;
            ids.push(id);
        }

        Ok(ids)
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new("default")
    }
}

/// Queue-related errors
#[derive(Debug, Clone)]
pub enum QueueError {
    /// Invalid command configuration
    InvalidCommand(String),
    /// Queue not available
    QueueUnavailable(String),
    /// Job already exists
    DuplicateJob(String),
    /// Failed to dispatch
    DispatchFailed(String),
    /// Other errors
    Other(String),
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueError::InvalidCommand(msg) => write!(f, "Invalid command: {}", msg),
            QueueError::QueueUnavailable(queue) => write!(f, "Queue '{}' is unavailable", queue),
            QueueError::DuplicateJob(id) => write!(f, "Job '{}' already exists", id),
            QueueError::DispatchFailed(msg) => write!(f, "Failed to dispatch: {}", msg),
            QueueError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for QueueError {}

/// Queue manager for managing multiple queues
pub struct QueueManager {
    queues: std::collections::HashMap<String, CommandQueue>,
}

impl QueueManager {
    /// Create a new queue manager
    pub fn new() -> Self {
        Self {
            queues: std::collections::HashMap::new(),
        }
    }

    /// Add a queue
    pub fn add_queue(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.queues.insert(name.clone(), CommandQueue::new(name));
    }

    /// Get a queue
    pub fn queue(&self, name: &str) -> Option<&CommandQueue> {
        self.queues.get(name)
    }

    /// Get or create a queue
    pub fn queue_or_create(&mut self, name: impl Into<String>) -> CommandQueue {
        let name = name.into();
        self.queues
            .entry(name.clone())
            .or_insert_with(|| CommandQueue::new(name.clone()))
            .clone()
    }

    /// List all queues
    pub fn list(&self) -> Vec<&str> {
        self.queues.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        let mut manager = Self::new();
        manager.add_queue("default");
        manager
    }
}

/// Job dispatch result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobDispatch {
    /// Job ID
    pub job_id: String,
    /// Command name
    pub command: String,
    /// Queue name
    pub queue: String,
    /// Dispatch timestamp
    pub dispatched_at: String,
    /// Scheduled execution time
    pub scheduled_at: Option<String>,
}

impl JobDispatch {
    /// Create a new job dispatch result
    pub fn new(
        job_id: String,
        command: String,
        queue: String,
    ) -> Self {
        Self {
            job_id,
            command,
            queue,
            dispatched_at: chrono::Utc::now().to_rfc3339(),
            scheduled_at: None,
        }
    }

    /// Set the scheduled execution time
    pub fn with_scheduled_at(mut self, scheduled_at: String) -> Self {
        self.scheduled_at = Some(scheduled_at);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queued_command_creation() {
        let cmd = QueuedCommand::new("test");
        assert_eq!(cmd.command, "test");
        assert!(!cmd.job_id.is_empty());
    }

    #[test]
    fn test_queued_command_builder() {
        let cmd = QueuedCommand::new("import:data")
            .with_args(vec!["file.csv".to_string()])
            .with_delay(Duration::from_secs(60))
            .with_max_attempts(5);

        assert_eq!(cmd.command, "import:data");
        assert_eq!(cmd.args.len(), 1);
        assert!(cmd.is_delayed());
        assert_eq!(cmd.max_attempts, 5);
    }

    #[test]
    fn test_command_queue_default() {
        let queue = CommandQueue::default();
        assert_eq!(queue.name(), "default");
    }

    #[test]
    fn test_queue_manager() {
        let mut manager = QueueManager::new();
        manager.add_queue("background");

        assert!(manager.queue("default").is_some());
        assert!(manager.queue("background").is_some());
    }

    #[test]
    fn test_queue_manager_list() {
        let manager = QueueManager::default();
        let queues = manager.list();
        assert!(queues.contains(&"default"));
    }

    #[tokio::test]
    async fn test_queue_dispatch() {
        let queue = CommandQueue::default();
        let cmd = QueuedCommand::new("test");

        let result = queue.dispatch(cmd).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_queue_dispatch_invalid_command() {
        let queue = CommandQueue::default();
        let cmd = QueuedCommand::new("");

        let result = queue.dispatch(cmd).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_job_dispatch_result() {
        let dispatch = JobDispatch::new(
            "job123".to_string(),
            "test".to_string(),
            "default".to_string(),
        );

        assert_eq!(dispatch.job_id, "job123");
        assert_eq!(dispatch.command, "test");
        assert_eq!(dispatch.queue, "default");
    }

    #[test]
    fn test_delay_seconds() {
        let cmd = QueuedCommand::new("test")
            .with_delay(Duration::from_secs(120));

        assert_eq!(cmd.delay_seconds(), 120);
    }
}
