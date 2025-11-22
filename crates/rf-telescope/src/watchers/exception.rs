//! Exception watcher for error and exception tracking

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Exception information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionInfo {
    pub exception_type: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub stack_trace: Vec<String>,
    pub context: HashMap<String, String>,
    pub request_path: Option<String>,
    pub user_id: Option<String>,
    pub occurred_at: DateTime<Utc>,
}

impl ExceptionInfo {
    /// Create a new exception info
    pub fn new(exception_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            exception_type: exception_type.into(),
            message: message.into(),
            file: None,
            line: None,
            stack_trace: Vec::new(),
            context: HashMap::new(),
            request_path: None,
            user_id: None,
            occurred_at: Utc::now(),
        }
    }

    /// Set file and line number
    pub fn with_location(mut self, file: impl Into<String>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    /// Add a stack trace line
    pub fn add_stack_line(mut self, line: impl Into<String>) -> Self {
        self.stack_trace.push(line.into());
        self
    }

    /// Set the stack trace
    pub fn with_stack_trace(mut self, stack_trace: Vec<String>) -> Self {
        self.stack_trace = stack_trace;
        self
    }

    /// Add context information
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Set request path
    pub fn with_request(mut self, path: impl Into<String>) -> Self {
        self.request_path = Some(path.into());
        self
    }

    /// Set user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

/// Exception watcher for monitoring errors and exceptions
#[derive(Clone)]
pub struct ExceptionWatcher {
    storage: Storage,
}

impl ExceptionWatcher {
    /// Create a new exception watcher
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Record an exception
    pub async fn record(&self, info: ExceptionInfo) {
        let entry = Entry::new(
            EntryType::Exception,
            json!({
                "exception_type": info.exception_type,
                "message": info.message,
                "file": info.file,
                "line": info.line,
                "stack_trace": info.stack_trace,
                "context": info.context,
                "request_path": info.request_path,
                "user_id": info.user_id,
                "occurred_at": info.occurred_at,
            }),
        )
        .with_tag(format!("type:{}", info.exception_type));

        if let Some(ref path) = info.request_path {
            self.storage
                .store(entry.with_tag(format!("path:{}", path)))
                .await;
        } else {
            self.storage.store(entry).await;
        }
    }

    /// Get all recorded exceptions
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Exception).await
    }

    /// Get exceptions by type
    pub async fn by_type(&self, exception_type: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("type:{}", exception_type);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }

    /// Get exceptions for a specific request path
    pub async fn by_path(&self, path: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("path:{}", path);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exception_info_creation() {
        let info = ExceptionInfo::new("RuntimeError", "Something went wrong")
            .with_location("main.rs", 42)
            .with_context("user_action", "delete_account");

        assert_eq!(info.exception_type, "RuntimeError");
        assert_eq!(info.message, "Something went wrong");
        assert_eq!(info.file, Some("main.rs".to_string()));
        assert_eq!(info.line, Some(42));
        assert_eq!(info.context.get("user_action").unwrap(), "delete_account");
    }

    #[tokio::test]
    async fn test_exception_watcher_record() {
        let storage = Storage::new();
        let watcher = ExceptionWatcher::new(storage);

        let info = ExceptionInfo::new("DatabaseError", "Connection failed")
            .with_stack_trace(vec!["at main.rs:10".to_string(), "at db.rs:45".to_string()]);

        watcher.record(info).await;

        let exceptions = watcher.all().await;
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].content["exception_type"], "DatabaseError");
        assert_eq!(exceptions[0].content["message"], "Connection failed");
    }

    #[tokio::test]
    async fn test_exception_by_type() {
        let storage = Storage::new();
        let watcher = ExceptionWatcher::new(storage);

        watcher
            .record(ExceptionInfo::new("RuntimeError", "Error 1"))
            .await;
        watcher
            .record(ExceptionInfo::new("DatabaseError", "Error 2"))
            .await;
        watcher
            .record(ExceptionInfo::new("RuntimeError", "Error 3"))
            .await;

        let runtime_errors = watcher.by_type("RuntimeError").await;
        assert_eq!(runtime_errors.len(), 2);
    }

    #[tokio::test]
    async fn test_exception_by_path() {
        let storage = Storage::new();
        let watcher = ExceptionWatcher::new(storage);

        watcher
            .record(ExceptionInfo::new("Error", "Test").with_request("/api/users"))
            .await;
        watcher
            .record(ExceptionInfo::new("Error", "Test").with_request("/api/posts"))
            .await;

        let errors = watcher.by_path("/api/users").await;
        assert_eq!(errors.len(), 1);
    }

    #[tokio::test]
    async fn test_exception_with_full_context() {
        let storage = Storage::new();
        let watcher = ExceptionWatcher::new(storage);

        let info = ExceptionInfo::new("ValidationError", "Invalid input")
            .with_location("validator.rs", 123)
            .with_stack_trace(vec![
                "at validator.rs:123".to_string(),
                "at main.rs:45".to_string(),
            ])
            .with_context("field", "email")
            .with_context("value", "invalid-email")
            .with_request("/api/register")
            .with_user("user123");

        watcher.record(info).await;

        let exceptions = watcher.all().await;
        assert_eq!(exceptions.len(), 1);
        assert_eq!(exceptions[0].content["message"], "Invalid input");
        assert_eq!(exceptions[0].content["user_id"], "user123");
    }
}
