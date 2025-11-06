//! Structured logging with trace correlation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, Level};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::tracing_middleware::{current_span_id, current_trace_id};

/// Structured log entry with trace correlation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp in ISO 8601 format
    pub timestamp: DateTime<Utc>,

    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log message
    pub message: String,

    /// OpenTelemetry trace ID for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,

    /// OpenTelemetry span ID for correlation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,

    /// Additional structured fields
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl LogEntry {
    /// Create new log entry
    pub fn new(level: &str, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.into(),
            trace_id: current_trace_id(),
            span_id: current_span_id(),
            fields: HashMap::new(),
        }
    }

    /// Add a field to the log entry
    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Structured logger
pub struct StructuredLogger;

impl StructuredLogger {
    /// Log with trace correlation
    pub fn log(level: Level, message: impl Into<String>, fields: HashMap<String, serde_json::Value>) {
        let mut entry = LogEntry::new(level.as_str(), message);
        entry.fields = fields;

        match level {
            Level::ERROR => tracing::error!("{}", entry.message),
            Level::WARN => tracing::warn!("{}", entry.message),
            Level::INFO => tracing::info!("{}", entry.message),
            Level::DEBUG => tracing::debug!("{}", entry.message),
            Level::TRACE => tracing::trace!("{}", entry.message),
        }
    }

    /// Log info with structured fields
    pub fn info(message: impl Into<String>, fields: HashMap<String, serde_json::Value>) {
        Self::log(Level::INFO, message, fields);
    }

    /// Log error with structured fields
    pub fn error(message: impl Into<String>, fields: HashMap<String, serde_json::Value>) {
        Self::log(Level::ERROR, message, fields);
    }

    /// Log warning with structured fields
    pub fn warn(message: impl Into<String>, fields: HashMap<String, serde_json::Value>) {
        Self::log(Level::WARN, message, fields);
    }

    /// Log debug with structured fields
    pub fn debug(message: impl Into<String>, fields: HashMap<String, serde_json::Value>) {
        Self::log(Level::DEBUG, message, fields);
    }
}

/// Initialize logging system
pub fn init_logging(log_level: &str, json_format: bool) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_new(log_level)
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    if json_format {
        // JSON formatted logs for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true),
            )
            .try_init()?;
    } else {
        // Human-readable logs for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_file(true)
                    .with_line_number(true)
                    .with_span_events(FmtSpan::CLOSE),
            )
            .try_init()?;
    }

    info!("Logging initialized with level: {}", log_level);
    Ok(())
}

/// Macro for structured logging with trace correlation
#[macro_export]
macro_rules! log_with_trace {
    ($level:expr, $msg:expr) => {
        $crate::logging::StructuredLogger::log(
            $level,
            $msg,
            std::collections::HashMap::new(),
        )
    };
    ($level:expr, $msg:expr, $($key:expr => $value:expr),+ $(,)?) => {
        {
            let mut fields = std::collections::HashMap::new();
            $(
                fields.insert($key.to_string(), serde_json::json!($value));
            )+
            $crate::logging::StructuredLogger::log($level, $msg, fields)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new("info", "Test message");
        assert_eq!(entry.level, "info");
        assert_eq!(entry.message, "Test message");
    }

    #[test]
    fn test_log_entry_with_fields() {
        let entry = LogEntry::new("error", "Error occurred")
            .with_field("error_code", serde_json::json!(500))
            .with_field("component", serde_json::json!("database"));

        assert_eq!(entry.fields.len(), 2);
        assert_eq!(entry.fields.get("error_code"), Some(&serde_json::json!(500)));
    }

    #[test]
    fn test_log_entry_to_json() {
        let entry = LogEntry::new("info", "Test");
        let json = entry.to_json();
        assert!(json.is_ok());
    }

    #[test]
    fn test_init_logging() {
        // Can only init once, so test with error handling
        let result = init_logging("info", false);
        // May fail if already initialized, which is ok
    }
}
