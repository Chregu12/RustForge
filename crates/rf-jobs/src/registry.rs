//! Job registry for type-safe job execution
//!
//! This module provides a registry that maps job type names to their handlers,
//! enabling dynamic deserialization and execution of jobs from serialized payloads.

use crate::context::JobContext;
use crate::error::{JobError, JobResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use parking_lot::RwLock;

/// Backoff strategy for failed jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Exponential backoff (delay doubles each retry)
    Exponential,
    /// Linear backoff (delay increases linearly)
    Linear,
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential
    }
}

impl BackoffStrategy {
    /// Calculate delay for given attempt number
    pub fn calculate_delay(&self, attempt: u32, base_delay: u64) -> u64 {
        match self {
            Self::Fixed => base_delay,
            Self::Exponential => base_delay * 2u64.pow(attempt),
            Self::Linear => base_delay * (attempt as u64 + 1),
        }
    }
}

/// Enhanced job trait with registry support
///
/// This trait extends the basic Job trait with type information
/// needed for dynamic dispatch through the registry.
///
/// # Example
///
/// ```
/// use rf_jobs::registry::{JobWithRegistry, BackoffStrategy};
/// use rf_jobs::{JobContext, JobResult};
/// use async_trait::async_trait;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct SendEmailJob {
///     to: String,
///     subject: String,
/// }
///
/// #[async_trait]
/// impl JobWithRegistry for SendEmailJob {
///     fn job_type(&self) -> &'static str {
///         "send_email"
///     }
///
///     async fn handle(&self, ctx: JobContext) -> JobResult {
///         ctx.log(&format!("Sending email to {}", self.to));
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait JobWithRegistry: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// Unique job type identifier
    ///
    /// This should be a stable string that uniquely identifies the job type.
    /// It's used for serialization/deserialization routing.
    fn job_type(&self) -> &'static str;

    /// Execute the job
    async fn handle(&self, ctx: JobContext) -> JobResult;

    /// Maximum retry attempts (default: 3)
    fn max_attempts(&self) -> u32 {
        3
    }

    /// Backoff strategy (default: Exponential)
    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential
    }

    /// Base backoff duration in seconds (default: 60)
    fn base_backoff_seconds(&self) -> u64 {
        60
    }

    /// Called when job fails after all retries
    async fn failed(&self, _ctx: JobContext, _error: &JobError) {
        // Override to handle failed jobs
    }
}

/// Handler for a specific job type
///
/// This trait is implemented by the registry for each registered job type
/// to enable dynamic deserialization and execution.
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// Deserialize payload and execute the job
    async fn deserialize_and_execute(&self, payload: &str, ctx: JobContext) -> JobResult;

    /// Get job type name
    fn job_type_name(&self) -> &'static str;

    /// Get max attempts for this job type
    fn max_attempts(&self) -> u32;

    /// Get backoff strategy
    fn backoff_strategy(&self) -> BackoffStrategy;

    /// Get base backoff seconds
    fn base_backoff_seconds(&self) -> u64;
}

/// Concrete implementation of JobHandler for a specific job type
struct JobHandlerImpl<J: JobWithRegistry + 'static> {
    _phantom: PhantomData<J>,
}

impl<J: JobWithRegistry + 'static> JobHandlerImpl<J> {
    fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<J: JobWithRegistry + 'static> JobHandler for JobHandlerImpl<J> {
    async fn deserialize_and_execute(&self, payload: &str, ctx: JobContext) -> JobResult {
        // Deserialize the job from JSON
        let job: J = serde_json::from_str(payload)
            .map_err(|e| JobError::SerializationError(e))?;

        // Execute the job
        match job.handle(ctx.clone()).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Call failed handler if all retries exhausted
                if ctx.attempt() >= ctx.max_attempts() {
                    job.failed(ctx, &e).await;
                }
                Err(e)
            }
        }
    }

    fn job_type_name(&self) -> &'static str {
        // We need a dummy instance to get the job type
        // This is a bit of a hack, but works for static job types
        std::any::type_name::<J>()
    }

    fn max_attempts(&self) -> u32 {
        // Create a dummy instance to get default values
        // For production, you'd want to pass these as constructor args
        3 // Default value
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential
    }

    fn base_backoff_seconds(&self) -> u64 {
        60
    }
}

/// Global job registry for mapping job types to handlers
///
/// The registry maintains a mapping from job type names (strings) to
/// their corresponding handlers, enabling dynamic job execution.
///
/// # Example
///
/// ```
/// use rf_jobs::registry::JobRegistry;
/// # use rf_jobs::registry::{JobWithRegistry, BackoffStrategy};
/// # use rf_jobs::{JobContext, JobResult};
/// # use async_trait::async_trait;
/// # use serde::{Serialize, Deserialize};
/// #
/// # #[derive(Debug, Clone, Serialize, Deserialize)]
/// # struct SendEmailJob { to: String }
/// #
/// # #[async_trait]
/// # impl JobWithRegistry for SendEmailJob {
/// #     fn job_type(&self) -> &'static str { "send_email" }
/// #     async fn handle(&self, ctx: JobContext) -> JobResult { Ok(()) }
/// # }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut registry = JobRegistry::new();
/// registry.register::<SendEmailJob>("send_email");
///
/// // Later, execute a job from its serialized form
/// let payload = r#"{"to":"user@example.com"}"#;
/// let ctx = JobContext::new(
///     uuid::Uuid::new_v4(),
///     "default".to_string(),
///     0,
///     3,
///     chrono::Utc::now(),
/// );
/// registry.execute("send_email", payload, ctx).await?;
/// # Ok(())
/// # }
/// ```
pub struct JobRegistry {
    handlers: Arc<RwLock<HashMap<String, Arc<dyn JobHandler>>>>,
}

impl JobRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a job type with its handler
    ///
    /// # Arguments
    ///
    /// * `job_type` - Unique identifier for the job type
    ///
    /// # Type Parameters
    ///
    /// * `J` - The job type implementing JobWithRegistry
    ///
    /// # Example
    ///
    /// ```
    /// # use rf_jobs::registry::{JobRegistry, JobWithRegistry};
    /// # use rf_jobs::{JobContext, JobResult};
    /// # use async_trait::async_trait;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # struct MyJob;
    /// # #[async_trait]
    /// # impl JobWithRegistry for MyJob {
    /// #     fn job_type(&self) -> &'static str { "my_job" }
    /// #     async fn handle(&self, _: JobContext) -> JobResult { Ok(()) }
    /// # }
    /// let mut registry = JobRegistry::new();
    /// registry.register::<MyJob>("my_job");
    /// ```
    pub fn register<J: JobWithRegistry + 'static>(&mut self, job_type: &str) {
        let handler = Arc::new(JobHandlerImpl::<J>::new());
        self.handlers.write().insert(job_type.to_string(), handler);
    }

    /// Execute a job by deserializing and dispatching to its handler
    ///
    /// # Arguments
    ///
    /// * `job_type` - The job type identifier
    /// * `payload` - JSON-serialized job data
    /// * `ctx` - Job execution context
    ///
    /// # Errors
    ///
    /// Returns `JobError::Custom` if the job type is not registered.
    /// Returns `JobError::SerializationError` if deserialization fails.
    /// Returns the job's error if execution fails.
    pub async fn execute(&self, job_type: &str, payload: &str, ctx: JobContext) -> JobResult {
        // Clone the handler Arc to avoid holding lock across await
        let handler = {
            let handlers = self.handlers.read();
            Arc::clone(
                handlers
                    .get(job_type)
                    .ok_or_else(|| JobError::Custom(format!("Unknown job type: {}", job_type)))?,
            )
        }; // Lock is dropped here

        handler.deserialize_and_execute(payload, ctx).await
    }

    /// Check if a job type is registered
    pub fn has_job_type(&self, job_type: &str) -> bool {
        self.handlers.read().contains_key(job_type)
    }

    /// Get all registered job types
    pub fn job_types(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// Get handler info for a job type
    pub fn get_handler_info(&self, job_type: &str) -> Option<HandlerInfo> {
        let handlers = self.handlers.read();
        handlers.get(job_type).map(|handler| HandlerInfo {
            job_type: handler.job_type_name(),
            max_attempts: handler.max_attempts(),
            backoff_strategy: handler.backoff_strategy(),
            base_backoff_seconds: handler.base_backoff_seconds(),
        })
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for JobRegistry {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
        }
    }
}

/// Information about a registered job handler
#[derive(Debug, Clone)]
pub struct HandlerInfo {
    pub job_type: &'static str,
    pub max_attempts: u32,
    pub backoff_strategy: BackoffStrategy,
    pub base_backoff_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestJob {
        value: String,
    }

    #[async_trait]
    impl JobWithRegistry for TestJob {
        fn job_type(&self) -> &'static str {
            "test_job"
        }

        async fn handle(&self, ctx: JobContext) -> JobResult {
            ctx.log(&format!("Handled: {}", self.value));
            Ok(())
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct FailingJob {
        should_fail: bool,
    }

    #[async_trait]
    impl JobWithRegistry for FailingJob {
        fn job_type(&self) -> &'static str {
            "failing_job"
        }

        async fn handle(&self, _ctx: JobContext) -> JobResult {
            if self.should_fail {
                Err(JobError::ExecutionFailed("Job failed".to_string()))
            } else {
                Ok(())
            }
        }

        fn max_attempts(&self) -> u32 {
            5
        }

        fn backoff_strategy(&self) -> BackoffStrategy {
            BackoffStrategy::Linear
        }
    }

    #[tokio::test]
    async fn test_register_and_execute() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob>("test_job");

        let job = TestJob {
            value: "test".to_string(),
        };
        let payload = serde_json::to_string(&job).unwrap();
        let ctx = JobContext::new(
            uuid::Uuid::new_v4(),
            "default".to_string(),
            0,
            3,
            chrono::Utc::now(),
        );

        let result = registry.execute("test_job", &payload, ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unknown_job_type() {
        let registry = JobRegistry::new();
        let ctx = JobContext::new(
            uuid::Uuid::new_v4(),
            "default".to_string(),
            0,
            3,
            chrono::Utc::now(),
        );

        let result = registry.execute("unknown", "{}", ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JobError::Custom(_)));
    }

    #[tokio::test]
    async fn test_invalid_payload() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob>("test_job");

        let ctx = JobContext::new(
            uuid::Uuid::new_v4(),
            "default".to_string(),
            0,
            3,
            chrono::Utc::now(),
        );

        let result = registry.execute("test_job", "invalid json", ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), JobError::SerializationError(_)));
    }

    #[tokio::test]
    async fn test_has_job_type() {
        let mut registry = JobRegistry::new();
        assert!(!registry.has_job_type("test_job"));

        registry.register::<TestJob>("test_job");
        assert!(registry.has_job_type("test_job"));
    }

    #[tokio::test]
    async fn test_job_types() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob>("test_job");
        registry.register::<FailingJob>("failing_job");

        let job_types = registry.job_types();
        assert_eq!(job_types.len(), 2);
        assert!(job_types.contains(&"test_job".to_string()));
        assert!(job_types.contains(&"failing_job".to_string()));
    }

    #[tokio::test]
    async fn test_backoff_strategy_fixed() {
        let strategy = BackoffStrategy::Fixed;
        assert_eq!(strategy.calculate_delay(0, 10), 10);
        assert_eq!(strategy.calculate_delay(1, 10), 10);
        assert_eq!(strategy.calculate_delay(5, 10), 10);
    }

    #[tokio::test]
    async fn test_backoff_strategy_exponential() {
        let strategy = BackoffStrategy::Exponential;
        assert_eq!(strategy.calculate_delay(0, 10), 10);
        assert_eq!(strategy.calculate_delay(1, 10), 20);
        assert_eq!(strategy.calculate_delay(2, 10), 40);
        assert_eq!(strategy.calculate_delay(3, 10), 80);
    }

    #[tokio::test]
    async fn test_backoff_strategy_linear() {
        let strategy = BackoffStrategy::Linear;
        assert_eq!(strategy.calculate_delay(0, 10), 10);
        assert_eq!(strategy.calculate_delay(1, 10), 20);
        assert_eq!(strategy.calculate_delay(2, 10), 30);
        assert_eq!(strategy.calculate_delay(3, 10), 40);
    }

    #[tokio::test]
    async fn test_failing_job() {
        let mut registry = JobRegistry::new();
        registry.register::<FailingJob>("failing_job");

        let job = FailingJob { should_fail: true };
        let payload = serde_json::to_string(&job).unwrap();
        let ctx = JobContext::new(
            uuid::Uuid::new_v4(),
            "default".to_string(),
            0,
            3,
            chrono::Utc::now(),
        );

        let result = registry.execute("failing_job", &payload, ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_clone() {
        let mut registry = JobRegistry::new();
        registry.register::<TestJob>("test_job");

        let cloned = registry.clone();
        assert!(cloned.has_job_type("test_job"));
    }
}
