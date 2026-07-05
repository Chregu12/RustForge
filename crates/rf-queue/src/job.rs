//! Job trait and types

use crate::error::QueueError;
use crate::queue::Queue;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Job trait for queue jobs
///
/// Implement this trait to create custom jobs that can be dispatched to the queue.
///
/// # Example
///
/// ```
/// use rf_queue::{Job, QueueError};
/// use async_trait::async_trait;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct SendEmailJob {
///     to: String,
///     subject: String,
///     body: String,
/// }
///
/// #[async_trait]
/// impl Job for SendEmailJob {
///     async fn handle(&self) -> Result<(), QueueError> {
///         // Send email logic here
///         println!("Sending email to {}", self.to);
///         Ok(())
///     }
///
///     fn job_type(&self) -> &'static str {
///         "send_email"
///     }
/// }
/// ```
#[async_trait]
pub trait Job: Send + Sync + Serialize + for<'de> Deserialize<'de> {
    /// Execute the job
    async fn handle(&self) -> Result<(), QueueError>;

    /// Get the job type identifier
    fn job_type(&self) -> &'static str;

    /// Maximum number of retry attempts
    fn max_retries(&self) -> u32 {
        3
    }

    /// Timeout for job execution
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Queue name (default: "default")
    fn queue(&self) -> &str {
        "default"
    }

    /// Job priority (higher = more important)
    fn priority(&self) -> i32 {
        0
    }

    /// Ergonomically dispatch this job onto a queue.
    ///
    /// This is the concise form envisioned as `SendInvoice { .. }.dispatch(&queue)`:
    /// it serializes the job into a [`JobMetadata`] and enqueues it. A [`Worker`]
    /// registered for this job type (see [`Worker::register`]) will later dequeue
    /// and execute [`Job::handle`].
    ///
    /// [`Worker`]: crate::worker::Worker
    /// [`Worker::register`]: crate::worker::Worker::register
    async fn dispatch(&self, queue: &Arc<dyn Queue>) -> Result<String, QueueError>
    where
        Self: Sized,
    {
        let metadata = JobMetadata::new(self)?;
        queue.push(metadata).await
    }

    /// Ergonomically dispatch this job onto the **process-global default queue**
    /// with no queue handle threaded through the call site.
    ///
    /// This is the concise, handle-free form envisioned as
    /// `SendInvoice { .. }.dispatch_now()`. The default queue is configured once
    /// at boot with [`set_default_queue`](crate::set_default_queue) /
    /// [`Jobs::set_queue`](crate::Jobs::set_queue) (until then it is an in-memory
    /// queue).
    ///
    /// Unlike [`Job::dispatch`], this is **synchronous** and drives the async
    /// enqueue on the deadlock-safe [`AsyncBridge`](rf_async_bridge::AsyncBridge),
    /// so it is safe to call from plain sync code *and* from inside an existing
    /// Tokio runtime (an Axum handler, a spawned task, `#[tokio::main]`) without
    /// a "runtime within a runtime" panic.
    fn dispatch_now(&self) -> Result<String, QueueError>
    where
        Self: Sized,
    {
        let metadata = JobMetadata::new(self)?;
        crate::facade::push_to_default(metadata)
    }
}

/// The user-authored execution body of a job, used with `#[derive(Job)]`.
///
/// The full [`Job`] trait is mostly mechanical: `job_type()` is the type name,
/// and `queue`/`max_retries`/`timeout`/`priority` are configuration with sane
/// defaults. The only part that genuinely needs a user body is [`Job::handle`].
///
/// `#[derive(Job)]` generates the entire [`Job`] impl for you — deriving
/// `job_type` from the struct name and reading `queue`/`retries`/`timeout`/
/// `priority` from a terse `#[job(..)]` attribute — and delegates the body to
/// this trait. You therefore only write the one method that matters:
///
/// ```
/// use rf_queue::{Job, JobHandler, QueueError};
/// use async_trait::async_trait;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Job)]
/// #[job(queue = "emails", retries = 5)]
/// struct SendEmail { to: String }
///
/// #[async_trait]
/// impl JobHandler for SendEmail {
///     async fn handle(&self) -> Result<(), QueueError> {
///         println!("emailing {}", self.to);
///         Ok(())
///     }
/// }
///
/// // The derive wired the rest: job_type == "SendEmail", queue == "emails",
/// // max_retries == 5, and dispatch_now()/Worker::register still work.
/// assert_eq!(SendEmail { to: "a@b.c".into() }.job_type(), "SendEmail");
/// assert_eq!(SendEmail { to: "a@b.c".into() }.queue(), "emails");
/// ```
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// Execute the job body. This is the only method you implement by hand when
    /// using `#[derive(Job)]`; the derive supplies the rest of [`Job`].
    async fn handle(&self) -> Result<(), QueueError>;
}

/// Job metadata stored in queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    /// Unique job ID
    pub id: String,

    /// Job type identifier
    pub job_type: String,

    /// Concrete Rust type key used to route the job to a worker handler.
    ///
    /// This is the stable `std::any::type_name::<J>()` of the dispatched job,
    /// which is how [`Worker::register`]/[`Worker::handle`] key their handlers.
    /// It is distinct from [`Self::job_type`] (a user-facing label that may
    /// collide across types). Defaults to empty for payloads serialized before
    /// this field existed, in which case the worker falls back to `job_type`.
    ///
    /// [`Worker::register`]: crate::worker::Worker::register
    /// [`Worker::handle`]: crate::worker::Worker::handle
    #[serde(default)]
    pub handler_key: String,

    /// Serialized job data
    pub data: Vec<u8>,

    /// Queue name
    pub queue: String,

    /// Number of attempts made
    pub attempts: u32,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Job priority
    pub priority: i32,

    /// Timeout in seconds
    pub timeout_secs: u64,

    /// When the job was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// When the job should be executed (for delayed jobs)
    pub execute_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Last error message
    pub last_error: Option<String>,
}

impl JobMetadata {
    /// Create new job metadata
    pub fn new<J: Job>(job: &J) -> Result<Self, QueueError> {
        let data =
            serde_json::to_vec(job).map_err(|e| QueueError::SerializationError(e.to_string()))?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            job_type: job.job_type().to_string(),
            handler_key: std::any::type_name::<J>().to_string(),
            data,
            queue: job.queue().to_string(),
            attempts: 0,
            max_retries: job.max_retries(),
            priority: job.priority(),
            timeout_secs: job.timeout().as_secs(),
            created_at: chrono::Utc::now(),
            execute_at: None,
            last_error: None,
        })
    }

    /// Create delayed job metadata
    pub fn new_delayed<J: Job>(job: &J, delay: Duration) -> Result<Self, QueueError> {
        let mut metadata = Self::new(job)?;
        metadata.execute_at = Some(chrono::Utc::now() + chrono::Duration::from_std(delay)
            .map_err(|e| QueueError::SerializationError(format!("Invalid delay duration: {}", e)))?);
        Ok(metadata)
    }

    /// Check if job should be executed now
    pub fn should_execute(&self) -> bool {
        if let Some(execute_at) = self.execute_at {
            chrono::Utc::now() >= execute_at
        } else {
            true
        }
    }

    /// Check if job can be retried
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_retries
    }

    /// Mark attempt
    pub fn mark_attempt(&mut self) {
        self.attempts += 1;
    }

    /// Mark error
    pub fn mark_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    /// Deserialize job data
    pub fn deserialize<J: Job>(&self) -> Result<J, QueueError> {
        serde_json::from_slice(&self.data)
            .map_err(|e| QueueError::DeserializationError(e.to_string()))
    }

    /// Convert to JSON bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, QueueError> {
        serde_json::to_vec(self).map_err(|e| QueueError::SerializationError(e.to_string()))
    }

    /// Create from JSON bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, QueueError> {
        serde_json::from_slice(data).map_err(|e| QueueError::DeserializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct TestJob {
        message: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test_job"
        }
    }

    #[test]
    fn test_job_metadata_creation() {
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();

        assert_eq!(metadata.job_type, "test_job");
        assert_eq!(metadata.queue, "default");
        assert_eq!(metadata.attempts, 0);
        assert_eq!(metadata.max_retries, 3);
    }

    #[test]
    fn test_job_serialization() {
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        let bytes = metadata.to_bytes().unwrap();
        let decoded = JobMetadata::from_bytes(&bytes).unwrap();

        assert_eq!(metadata.id, decoded.id);
        assert_eq!(metadata.job_type, decoded.job_type);
    }

    #[test]
    fn test_job_deserialization() {
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        let deserialized: TestJob = metadata.deserialize().unwrap();

        assert_eq!(job.message, deserialized.message);
    }

    #[test]
    fn test_delayed_job() {
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(60)).unwrap();

        assert!(metadata.execute_at.is_some());
        assert!(!metadata.should_execute());
    }

    #[test]
    fn test_retry_logic() {
        let job = TestJob {
            message: "test".to_string(),
        };

        let mut metadata = JobMetadata::new(&job).unwrap();

        assert!(metadata.can_retry());
        metadata.mark_attempt();
        assert_eq!(metadata.attempts, 1);
        assert!(metadata.can_retry());

        metadata.mark_attempt();
        metadata.mark_attempt();
        assert_eq!(metadata.attempts, 3);
        assert!(!metadata.can_retry());
    }
}
