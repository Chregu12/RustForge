use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Context information for event execution
#[derive(Debug, Clone)]
pub struct EventContext {
    pub command_name: String,
    pub args: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub output: String,
    pub metadata: HashMap<String, String>,
}

impl EventContext {
    pub fn new(command_name: impl Into<String>) -> Self {
        Self {
            command_name: command_name.into(),
            args: Vec::new(),
            start_time: Utc::now(),
            end_time: None,
            exit_code: None,
            output: String::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn finish(mut self, exit_code: i32, output: impl Into<String>) -> Self {
        self.end_time = Some(Utc::now());
        self.exit_code = Some(exit_code);
        self.output = output.into();
        self
    }

    pub fn duration(&self) -> Option<i64> {
        self.end_time.map(|end| (end - self.start_time).num_milliseconds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_context_creation() {
        let ctx = EventContext::new("make:model")
            .with_args(vec!["User".to_string()])
            .with_metadata("type", "model");

        assert_eq!(ctx.command_name, "make:model");
        assert_eq!(ctx.args.len(), 1);
        assert_eq!(ctx.metadata.get("type"), Some(&"model".to_string()));
    }

    #[test]
    fn test_event_context_finish() {
        let ctx = EventContext::new("test")
            .finish(0, "Success");

        assert_eq!(ctx.exit_code, Some(0));
        assert_eq!(ctx.output, "Success");
        assert!(ctx.end_time.is_some());
    }

    #[test]
    fn test_event_context_duration() {
        let ctx = EventContext::new("test")
            .finish(0, "Done");

        assert!(ctx.duration().is_some());
        assert!(ctx.duration().unwrap() >= 0);
    }
}
