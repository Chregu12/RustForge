//! Request context for tracking requests across the application.
//!
//! Provides `RequestContext` with trace IDs, path information, and environment detection.

use uuid::Uuid;

/// Application environment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    /// Development environment (verbose errors, debug info)
    Development,
    /// Staging environment
    Staging,
    /// Production environment (minimal error details)
    Production,
}

impl Environment {
    /// Detect environment from APP_ENV environment variable
    pub fn detect() -> Self {
        match std::env::var("APP_ENV").as_deref() {
            Ok("production") | Ok("prod") => Self::Production,
            Ok("staging") | Ok("stage") => Self::Staging,
            _ => Self::Development,
        }
    }
}

/// Request context for tracking individual requests
///
/// Contains trace ID for log correlation, request path, HTTP method,
/// and environment information.
///
/// # Example
///
/// ```rust
/// use rf_core::RequestContext;
///
/// let ctx = RequestContext::new("/api/users/123", "GET");
/// println!("Trace ID: {}", ctx.trace_id());
/// assert!(!ctx.is_production());
/// ```
#[derive(Clone, Debug)]
pub struct RequestContext {
    trace_id: String,
    path: String,
    method: String,
    environment: Environment,
}

impl RequestContext {
    /// Create a new request context
    ///
    /// Generates a unique trace ID (UUID v4) and detects the environment.
    ///
    /// # Arguments
    ///
    /// * `path` - Request path (e.g., "/api/users/123")
    /// * `method` - HTTP method (e.g., "GET", "POST")
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_core::RequestContext;
    ///
    /// let ctx = RequestContext::new("/api/users", "POST");
    /// assert_eq!(ctx.path(), "/api/users");
    /// assert_eq!(ctx.method(), "POST");
    /// ```
    pub fn new(path: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            path: path.into(),
            method: method.into(),
            environment: Environment::detect(),
        }
    }

    /// Create a context with a specific trace ID (for testing or propagation)
    pub fn with_trace_id(
        path: impl Into<String>,
        method: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            trace_id: trace_id.into(),
            path: path.into(),
            method: method.into(),
            environment: Environment::detect(),
        }
    }

    /// Get the trace ID
    ///
    /// The trace ID is unique per request and used for log correlation.
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Get the request path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the HTTP method
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Get the environment
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    /// Check if running in staging mode
    pub fn is_staging(&self) -> bool {
        self.environment == Environment::Staging
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("/api/users/123", "GET");

        assert_eq!(ctx.path(), "/api/users/123");
        assert_eq!(ctx.method(), "GET");
        assert!(!ctx.trace_id().is_empty());
    }

    #[test]
    fn test_request_context_with_trace_id() {
        let trace_id = "test-trace-123";
        let ctx = RequestContext::with_trace_id("/api/users", "POST", trace_id);

        assert_eq!(ctx.trace_id(), trace_id);
        assert_eq!(ctx.path(), "/api/users");
        assert_eq!(ctx.method(), "POST");
    }

    #[test]
    fn test_trace_id_is_unique() {
        let ctx1 = RequestContext::new("/api/users", "GET");
        let ctx2 = RequestContext::new("/api/users", "GET");

        assert_ne!(ctx1.trace_id(), ctx2.trace_id());
    }

    // Ignored: env vars can be affected by parallel tests from other crates
    // Run with: cargo test -p rf-core -- --ignored
    #[test]
    #[ignore = "env var race condition with parallel tests"]
    fn test_environment_detection_all() {
        // Test production
        std::env::set_var("APP_ENV", "production");
        assert_eq!(Environment::detect(), Environment::Production);

        // Test staging
        std::env::set_var("APP_ENV", "staging");
        assert_eq!(Environment::detect(), Environment::Staging);

        // Test development
        std::env::set_var("APP_ENV", "development");
        assert_eq!(Environment::detect(), Environment::Development);

        // Test default (remove env var)
        std::env::remove_var("APP_ENV");
        let ctx = RequestContext::new("/test", "GET");
        assert!(ctx.is_development());
        assert!(!ctx.is_production());
        assert!(!ctx.is_staging());
    }
}
