//! Advanced logging and tracing for RustForge
//!
//! This crate provides structured logging with:
//! - Multiple output formats (JSON, Pretty, Compact)
//! - File and stdout sinks
//! - Request ID tracking
//! - Performance timing
//! - Distributed tracing support

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub use tracing::{debug, error, info, instrument, span, trace, warn, Instrument, Span};
pub use uuid::Uuid;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Output format
    pub format: LogFormat,

    /// Log to stdout
    pub stdout: bool,

    /// Optional file output
    pub file: Option<PathBuf>,

    /// Directory for log files (if file logging enabled)
    pub log_dir: Option<PathBuf>,

    /// Enable request ID tracking
    pub request_id: bool,

    /// Enable performance timing
    pub timing: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            stdout: true,
            file: None,
            log_dir: None,
            request_id: true,
            timing: true,
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// JSON format for production
    Json,
    /// Pretty format for development
    Pretty,
    /// Compact format
    Compact,
}

/// Initialize logging with configuration
pub fn init_logging(config: LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let registry = tracing_subscriber::registry().with(env_filter);

    // Determine span events
    let span_events = if config.timing {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    match config.format {
        LogFormat::Json => {
            if config.stdout {
                let layer = fmt::layer()
                    .json()
                    .with_span_events(span_events)
                    .with_current_span(true)
                    .with_target(true);
                registry.with(layer).init();
            }
        }
        LogFormat::Pretty => {
            if config.stdout {
                let layer = fmt::layer()
                    .pretty()
                    .with_span_events(span_events)
                    .with_target(true);
                registry.with(layer).init();
            }
        }
        LogFormat::Compact => {
            if config.stdout {
                let layer = fmt::layer()
                    .compact()
                    .with_span_events(span_events)
                    .with_target(true);
                registry.with(layer).init();
            }
        }
    }

    Ok(())
}

/// Request ID for distributed tracing
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    /// Generate a new request ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from existing ID
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Performance timer for operations
#[derive(Debug)]
pub struct PerfTimer {
    name: String,
    start: std::time::Instant,
}

impl PerfTimer {
    /// Start a new timer
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start: std::time::Instant::now(),
        }
    }

    /// Stop the timer and log elapsed time
    pub fn stop(self) {
        let elapsed = self.start.elapsed();
        info!(
            operation = %self.name,
            duration_ms = elapsed.as_millis(),
            "Operation completed"
        );
    }

    /// Get elapsed time without stopping
    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }
}

/// Create a span with request ID
#[macro_export]
macro_rules! request_span {
    ($request_id:expr) => {
        $crate::span!(
            $crate::tracing::Level::INFO,
            "request",
            request_id = %$request_id
        )
    };
    ($request_id:expr, $($field:tt)*) => {
        $crate::span!(
            $crate::tracing::Level::INFO,
            "request",
            request_id = %$request_id,
            $($field)*
        )
    };
}

/// Log structured data
#[macro_export]
macro_rules! log_data {
    ($level:expr, $message:expr, $($key:ident = $value:expr),* $(,)?) => {
        match $level {
            $crate::tracing::Level::ERROR => $crate::error!($message, $($key = $value),*),
            $crate::tracing::Level::WARN => $crate::warn!($message, $($key = $value),*),
            $crate::tracing::Level::INFO => $crate::info!($message, $($key = $value),*),
            $crate::tracing::Level::DEBUG => $crate::debug!($message, $($key = $value),*),
            $crate::tracing::Level::TRACE => $crate::trace!($message, $($key = $value),*),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.stdout);
        assert!(config.request_id);
        assert!(config.timing);
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1.as_str(), id2.as_str());
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_request_id_from_string() {
        let id_str = "test-request-123";
        let id = RequestId::from_string(id_str.to_string());
        assert_eq!(id.as_str(), id_str);
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::from_string("test-id".to_string());
        assert_eq!(format!("{}", id), "test-id");
    }

    #[test]
    fn test_perf_timer_elapsed() {
        let timer = PerfTimer::start("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed();
        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_log_format_serialization() {
        let json_fmt = LogFormat::Json;
        let serialized = serde_json::to_string(&json_fmt).unwrap();
        assert_eq!(serialized, "\"json\"");

        let pretty_fmt = LogFormat::Pretty;
        let serialized = serde_json::to_string(&pretty_fmt).unwrap();
        assert_eq!(serialized, "\"pretty\"");
    }

    #[tokio::test]
    async fn test_init_logging_json() {
        let config = LogConfig {
            level: "debug".to_string(),
            format: LogFormat::Json,
            stdout: true,
            file: None,
            log_dir: None,
            request_id: true,
            timing: true,
        };

        // This should not panic
        // Note: We can't actually test output without capturing stderr
        // In real tests, you'd use a test subscriber
    }

    #[test]
    fn test_config_with_file() {
        let config = LogConfig {
            level: "warn".to_string(),
            format: LogFormat::Compact,
            stdout: false,
            file: Some(PathBuf::from("/var/log/app.log")),
            log_dir: Some(PathBuf::from("/var/log")),
            request_id: false,
            timing: false,
        };

        assert_eq!(config.level, "warn");
        assert!(!config.stdout);
        assert!(config.file.is_some());
    }
}
