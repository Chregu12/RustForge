//! Job trait and related types

use crate::context::JobContext;
use crate::error::{JobError, JobResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Trait for background jobs
///
/// # Example
///
/// ```
/// use rf_jobs::{Job, JobContext, JobResult};
/// use serde::{Deserialize, Serialize};
/// use async_trait::async_trait;
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct SendEmailJob {
///     to: String,
///     subject: String,
///     body: String,
/// }
///
/// #[async_trait]
/// impl Job for SendEmailJob {
///     async fn handle(&self, ctx: JobContext) -> JobResult {
///         ctx.log(&format!("Sending email to {}", self.to));
///         // Send email logic here
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Job: Send + Sync + Serialize + for<'de> Deserialize<'de> + Clone {
    /// Execute the job
    async fn handle(&self, ctx: JobContext) -> JobResult;

    /// Queue name (default: "default")
    fn queue(&self) -> &str {
        "default"
    }

    /// Maximum retry attempts (default: 3)
    fn max_attempts(&self) -> u32 {
        3
    }

    /// Backoff duration between retries (default: 60s)
    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Timeout for job execution (default: 60s)
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Called when job fails after all retries
    async fn failed(&self, _ctx: JobContext, _error: JobError) {
        // Override to handle failed jobs
    }
}

/// Job payload stored in queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPayload {
    /// Unique job ID
    pub id: uuid::Uuid,

    /// Queue name
    pub queue: String,

    /// Job type name
    pub job_type: String,

    /// Serialized job data
    pub data: serde_json::Value,

    /// Current attempt number
    pub attempt: u32,

    /// Maximum attempts
    pub max_attempts: u32,

    /// When job was dispatched
    pub dispatched_at: chrono::DateTime<chrono::Utc>,

    /// When job becomes available (for delayed jobs)
    pub available_at: chrono::DateTime<chrono::Utc>,

    /// Backoff duration (in seconds)
    pub backoff_seconds: u64,
}

impl JobPayload {
    /// Create new job payload
    pub fn new<J: Job>(job: J) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            queue: job.queue().to_string(),
            job_type: std::any::type_name::<J>().to_string(),
            data: serde_json::to_value(&job)?,
            attempt: 0,
            max_attempts: job.max_attempts(),
            dispatched_at: chrono::Utc::now(),
            available_at: chrono::Utc::now(),
            backoff_seconds: job.backoff().as_secs(),
        })
    }

    /// Deserialize job data
    pub fn deserialize<J: Job>(&self) -> Result<J, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }

    /// Increment attempt counter
    pub fn increment_attempt(&mut self) {
        self.attempt += 1;
    }

    /// Calculate next available time after failure
    pub fn next_available_at(&self) -> chrono::DateTime<chrono::Utc> {
        let backoff = chrono::Duration::seconds(self.backoff_seconds as i64);
        chrono::Utc::now() + backoff
    }

    /// Check if job has more attempts
    pub fn has_more_attempts(&self) -> bool {
        self.attempt < self.max_attempts
    }
}

/// Failed job entry for Dead Letter Queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedJob {
    /// Job payload
    pub payload: JobPayload,

    /// Error message
    pub error: String,

    /// When job failed
    pub failed_at: chrono::DateTime<chrono::Utc>,
}

impl FailedJob {
    /// Create new failed job entry
    pub fn new(payload: JobPayload, error: String) -> Self {
        Self {
            payload,
            error,
            failed_at: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        value: i32,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: JobContext) -> JobResult {
            Ok(())
        }
    }

    #[test]
    fn test_job_payload_creation() {
        let job = TestJob { value: 42 };
        let payload = JobPayload::new(job).unwrap();

        assert_eq!(payload.queue, "default");
        assert_eq!(payload.attempt, 0);
        assert_eq!(payload.max_attempts, 3);
        assert!(payload.has_more_attempts());
    }

    #[test]
    fn test_job_payload_deserialization() {
        let job = TestJob { value: 42 };
        let payload = JobPayload::new(job.clone()).unwrap();

        let deserialized: TestJob = payload.deserialize().unwrap();
        assert_eq!(deserialized.value, 42);
    }

    #[test]
    fn test_attempt_increment() {
        let job = TestJob { value: 42 };
        let mut payload = JobPayload::new(job).unwrap();

        assert_eq!(payload.attempt, 0);
        payload.increment_attempt();
        assert_eq!(payload.attempt, 1);
        assert!(payload.has_more_attempts());
    }

    #[test]
    fn test_max_attempts() {
        let job = TestJob { value: 42 };
        let mut payload = JobPayload::new(job).unwrap();
        payload.max_attempts = 2;

        payload.increment_attempt();
        assert!(payload.has_more_attempts());

        payload.increment_attempt();
        assert!(!payload.has_more_attempts());
    }
}
