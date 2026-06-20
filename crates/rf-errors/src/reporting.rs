//! Error reporting integration
//!
//! Provides error reporting to external services like Sentry with
//! configurable error levels and context attachment.

use crate::context::ErrorContext;
use crate::error::RustForgeError;
use async_trait::async_trait;
use std::sync::Arc;

/// Error reporter trait
///
/// Implement this trait to create custom error reporters.
#[async_trait]
pub trait ErrorReporter: Send + Sync {
    /// Report an error
    async fn report(&self, error: &RustForgeError, context: &ErrorContext);

    /// Report an error synchronously (if supported)
    fn report_sync(&self, error: &RustForgeError, context: &ErrorContext);

    /// Check if this reporter should report the given error
    fn should_report(&self, error: &RustForgeError) -> bool;
}

/// Error level for filtering which errors to report
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLevel {
    /// Only report critical errors
    Critical,
    /// Report errors and above
    Error,
    /// Report warnings and above
    Warning,
    /// Report all including debug
    Debug,
}

impl ErrorLevel {
    /// Check if we should report based on error status code
    pub fn should_report_status(&self, status_code: u16) -> bool {
        match self {
            Self::Critical => status_code >= 500,
            Self::Error => status_code >= 400,
            Self::Warning => status_code >= 300,
            Self::Debug => true,
        }
    }
}

/// Sentry error reporter
#[cfg(feature = "sentry-integration")]
pub struct SentryReporter {
    dsn: String,
    environment: String,
    release: Option<String>,
    level: ErrorLevel,
    _client: Arc<sentry::ClientOptions>,
}

#[cfg(feature = "sentry-integration")]
impl SentryReporter {
    /// Create a new Sentry reporter
    pub fn new(dsn: impl Into<String>, environment: impl Into<String>) -> Self {
        let dsn = dsn.into();
        let environment = environment.into();

        let client_options = sentry::ClientOptions {
            dsn: Some(dsn.parse().expect("Invalid Sentry DSN")),
            environment: Some(environment.clone().into()),
            ..Default::default()
        };

        Self {
            dsn,
            environment,
            release: None,
            level: ErrorLevel::Error,
            _client: Arc::new(client_options),
        }
    }

    /// Set release version
    pub fn with_release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    /// Set minimum error level to report
    pub fn with_level(mut self, level: ErrorLevel) -> Self {
        self.level = level;
        self
    }

    /// Initialize Sentry client
    pub fn init(&self) -> sentry::ClientInitGuard {
        let mut options = sentry::ClientOptions {
            dsn: Some(self.dsn.parse().expect("Invalid Sentry DSN")),
            environment: Some(self.environment.clone().into()),
            ..Default::default()
        };

        if let Some(ref release) = self.release {
            options.release = Some(release.clone().into());
        }

        sentry::init(options)
    }

    /// Convert RustForge error to Sentry level
    fn to_sentry_level(&self, error: &RustForgeError) -> sentry::Level {
        match error.status_code() {
            500..=599 => sentry::Level::Error,
            400..=499 => sentry::Level::Warning,
            _ => sentry::Level::Info,
        }
    }

    /// Attach context to Sentry scope
    fn attach_context_to_scope(&self, context: &ErrorContext, scope: &mut sentry::Scope) {
        // Set user context
        if let Some(ref user_id) = context.user_id {
            scope.set_user(Some(sentry::User {
                id: Some(user_id.clone()),
                ..Default::default()
            }));
        }

        // Set request context
        if let (Some(ref method), Some(ref path)) = (&context.method, &context.path) {
            scope.set_transaction(Some(path));
            scope.set_tag("http.method", method);
            scope.set_tag("http.path", path);
        }

        // Set request ID
        if let Some(ref request_id) = context.request_id {
            scope.set_tag("request_id", request_id);
        }

        // Set error ID
        scope.set_tag("error_id", &context.error_id);

        // Set environment
        scope.set_tag("environment", &context.environment);

        // Add custom context values (sanitized)
        for (key, value) in &context.values {
            if let Ok(string_value) = serde_json::to_string(value) {
                scope.set_extra(key, string_value.into());
            }
        }

        // Add tags
        for tag in &context.tags {
            scope.set_tag("tag", tag);
        }
    }
}

#[cfg(feature = "sentry-integration")]
#[async_trait]
impl ErrorReporter for SentryReporter {
    async fn report(&self, error: &RustForgeError, context: &ErrorContext) {
        if !self.should_report(error) {
            return;
        }

        sentry::with_scope(
            |scope| {
                self.attach_context_to_scope(context, scope);
                scope.set_level(Some(self.to_sentry_level(error)));
            },
            || {
                sentry::capture_message(
                    &format!("{} ({})", error, error.code().code()),
                    self.to_sentry_level(error),
                );
            },
        );
    }

    fn report_sync(&self, error: &RustForgeError, context: &ErrorContext) {
        if !self.should_report(error) {
            return;
        }

        sentry::with_scope(
            |scope| {
                self.attach_context_to_scope(context, scope);
                scope.set_level(Some(self.to_sentry_level(error)));
            },
            || {
                sentry::capture_message(
                    &format!("{} ({})", error, error.code().code()),
                    self.to_sentry_level(error),
                );
            },
        );
    }

    fn should_report(&self, error: &RustForgeError) -> bool {
        self.level.should_report_status(error.status_code())
    }
}

/// Logging error reporter (uses tracing)
pub struct LoggingReporter {
    level: ErrorLevel,
}

impl LoggingReporter {
    /// Create a new logging reporter
    pub fn new() -> Self {
        Self {
            level: ErrorLevel::Error,
        }
    }

    /// Set minimum error level to report
    pub fn with_level(mut self, level: ErrorLevel) -> Self {
        self.level = level;
        self
    }
}

impl Default for LoggingReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorReporter for LoggingReporter {
    async fn report(&self, error: &RustForgeError, context: &ErrorContext) {
        self.report_sync(error, context);
    }

    fn report_sync(&self, error: &RustForgeError, context: &ErrorContext) {
        if !self.should_report(error) {
            return;
        }

        // Log based on status code
        match error.status_code() {
            500..=599 => {
                tracing::error!(
                    error_id = %context.error_id,
                    error_code = %error.code().code(),
                    status = error.status_code(),
                    user_id = ?context.user_id,
                    path = ?context.path,
                    method = ?context.method,
                    "Error occurred: {}",
                    error
                );
            }
            400..=499 => {
                tracing::warn!(
                    error_id = %context.error_id,
                    error_code = %error.code().code(),
                    status = error.status_code(),
                    user_id = ?context.user_id,
                    path = ?context.path,
                    "Client error: {}",
                    error
                );
            }
            _ => {
                tracing::info!(
                    error_id = %context.error_id,
                    error_code = %error.code().code(),
                    "Error: {}",
                    error
                );
            }
        }
    }

    fn should_report(&self, error: &RustForgeError) -> bool {
        self.level.should_report_status(error.status_code())
    }
}

/// Multiple reporters aggregator
pub struct MultiReporter {
    reporters: Vec<Box<dyn ErrorReporter>>,
}

impl MultiReporter {
    /// Create a new multi-reporter
    pub fn new() -> Self {
        Self {
            reporters: Vec::new(),
        }
    }

    /// Add a reporter
    pub fn add_reporter(mut self, reporter: Box<dyn ErrorReporter>) -> Self {
        self.reporters.push(reporter);
        self
    }
}

impl Default for MultiReporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ErrorReporter for MultiReporter {
    async fn report(&self, error: &RustForgeError, context: &ErrorContext) {
        for reporter in &self.reporters {
            reporter.report(error, context).await;
        }
    }

    fn report_sync(&self, error: &RustForgeError, context: &ErrorContext) {
        for reporter in &self.reporters {
            reporter.report_sync(error, context);
        }
    }

    fn should_report(&self, error: &RustForgeError) -> bool {
        // Report if at least one reporter wants to report
        self.reporters.iter().any(|r| r.should_report(error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DatabaseError;

    #[test]
    fn test_error_level_should_report() {
        assert!(ErrorLevel::Critical.should_report_status(500));
        assert!(!ErrorLevel::Critical.should_report_status(404));

        assert!(ErrorLevel::Error.should_report_status(404));
        assert!(ErrorLevel::Error.should_report_status(500));
        assert!(!ErrorLevel::Error.should_report_status(301));

        assert!(ErrorLevel::Warning.should_report_status(301));
        assert!(ErrorLevel::Debug.should_report_status(200));
    }

    #[tokio::test]
    async fn test_logging_reporter() {
        let reporter = LoggingReporter::new();

        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        let ctx = ErrorContext::new();

        // Should not panic
        reporter.report(&err, &ctx).await;
    }

    #[tokio::test]
    async fn test_logging_reporter_should_report() {
        let reporter = LoggingReporter::new().with_level(ErrorLevel::Critical);

        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        assert!(reporter.should_report(&err)); // 500 error
    }

    #[tokio::test]
    async fn test_multi_reporter() {
        let reporter = MultiReporter::new().add_reporter(Box::new(LoggingReporter::new()));

        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);
        let ctx = ErrorContext::new();

        // Should not panic
        reporter.report(&err, &ctx).await;
    }

    #[cfg(feature = "sentry-integration")]
    #[test]
    fn test_sentry_reporter_creation() {
        let reporter = SentryReporter::new("https://key@sentry.io/project", "production")
            .with_release("1.0.0")
            .with_level(ErrorLevel::Error);

        assert_eq!(reporter.environment, "production");
        assert_eq!(reporter.release, Some("1.0.0".to_string()));
    }
}
