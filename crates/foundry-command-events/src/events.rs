use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;

/// Base trait for all events
pub trait Event: Send + Sync + 'static {
    /// Returns the event name
    fn event_name(&self) -> &'static str;

    /// Returns the event as Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Returns event metadata
    fn metadata(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

/// Event fired when a command is about to start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStarting {
    pub command: String,
    pub args: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub context: HashMap<String, String>,
}

impl Event for CommandStarting {
    fn event_name(&self) -> &'static str {
        "command.starting"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), self.command.clone());
        meta.insert("timestamp".to_string(), self.timestamp.to_rfc3339());
        meta
    }
}

/// Event fired when a command completes successfully
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFinished {
    pub command: String,
    pub duration: u64, // milliseconds
    pub exit_code: i32,
    pub output: String,
}

impl Event for CommandFinished {
    fn event_name(&self) -> &'static str {
        "command.finished"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), self.command.clone());
        meta.insert("duration".to_string(), self.duration.to_string());
        meta.insert("exit_code".to_string(), self.exit_code.to_string());
        meta
    }
}

/// Event fired when a command fails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFailed {
    pub command: String,
    pub error: String,
    pub exit_code: i32,
    pub duration: u64,
}

impl Event for CommandFailed {
    fn event_name(&self) -> &'static str {
        "command.failed"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), self.command.clone());
        meta.insert("error".to_string(), self.error.clone());
        meta.insert("exit_code".to_string(), self.exit_code.to_string());
        meta
    }
}

/// Event fired when a command receives SIGTERM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTerminated {
    pub command: String,
    pub signal: String,
    pub timestamp: DateTime<Utc>,
}

impl Event for CommandTerminated {
    fn event_name(&self) -> &'static str {
        "command.terminated"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("command".to_string(), self.command.clone());
        meta.insert("signal".to_string(), self.signal.clone());
        meta
    }
}

/// Custom event with user-defined data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEvent {
    pub name: String,
    pub data: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl Event for CustomEvent {
    fn event_name(&self) -> &'static str {
        "custom.event"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn metadata(&self) -> HashMap<String, String> {
        let mut meta = HashMap::new();
        meta.insert("name".to_string(), self.name.clone());
        meta.insert("timestamp".to_string(), self.timestamp.to_rfc3339());
        meta
    }
}

impl CustomEvent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_starting_event() {
        let event = CommandStarting {
            command: "make:model".to_string(),
            args: vec!["User".to_string()],
            timestamp: Utc::now(),
            context: HashMap::new(),
        };

        assert_eq!(event.event_name(), "command.starting");
        assert_eq!(event.command, "make:model");
    }

    #[test]
    fn test_command_finished_event() {
        let event = CommandFinished {
            command: "make:model".to_string(),
            duration: 150,
            exit_code: 0,
            output: "Success".to_string(),
        };

        assert_eq!(event.event_name(), "command.finished");
        assert_eq!(event.duration, 150);
    }

    #[test]
    fn test_command_failed_event() {
        let event = CommandFailed {
            command: "make:model".to_string(),
            error: "File already exists".to_string(),
            exit_code: 1,
            duration: 50,
        };

        assert_eq!(event.event_name(), "command.failed");
        assert_eq!(event.exit_code, 1);
    }

    #[test]
    fn test_custom_event() {
        let event = CustomEvent::new("user.registered")
            .with_data("user_id", serde_json::json!(123))
            .with_data("email", serde_json::json!("user@example.com"));

        assert_eq!(event.name, "user.registered");
        assert_eq!(event.data.len(), 2);
    }

    #[test]
    fn test_event_metadata() {
        let event = CommandFinished {
            command: "test".to_string(),
            duration: 100,
            exit_code: 0,
            output: "OK".to_string(),
        };

        let meta = event.metadata();
        assert_eq!(meta.get("command"), Some(&"test".to_string()));
        assert_eq!(meta.get("duration"), Some(&"100".to_string()));
    }
}
