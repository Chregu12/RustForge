//! Job serialization support
//!
//! This module provides utilities for serializing and deserializing jobs
//! to/from Redis-compatible formats.

use crate::error::JobError;
use crate::job::Job;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Serialized job representation for storage and transmission
///
/// This structure contains all the metadata and payload needed to
/// reconstruct and execute a job from its serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedJob {
    /// Unique job identifier
    pub id: uuid::Uuid,

    /// Job type identifier (used for registry lookup)
    pub job_type: String,

    /// Queue name
    pub queue: String,

    /// JSON-serialized job payload
    pub payload: serde_json::Value,

    /// Current attempt number (0-indexed)
    pub attempts: u32,

    /// Maximum retry attempts
    pub max_attempts: u32,

    /// Base backoff duration in seconds
    pub backoff_seconds: u64,

    /// When the job was originally queued
    pub queued_at: DateTime<Utc>,

    /// When the job becomes available for processing
    pub available_at: DateTime<Utc>,

    /// Job timeout in seconds
    pub timeout_seconds: u64,
}

impl SerializedJob {
    /// Create a serialized job from a Job instance
    ///
    /// # Arguments
    ///
    /// * `job` - The job to serialize
    ///
    /// # Errors
    ///
    /// Returns `JobError::SerializationError` if the job cannot be serialized to JSON.
    pub fn from_job<J: Job>(job: J) -> Result<Self, JobError> {
        let now = Utc::now();

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            job_type: std::any::type_name::<J>().to_string(),
            queue: job.queue().to_string(),
            payload: serde_json::to_value(&job).map_err(|e| JobError::SerializationError(e))?,
            attempts: 0,
            max_attempts: job.max_attempts(),
            backoff_seconds: job.backoff().as_secs(),
            queued_at: now,
            available_at: now,
            timeout_seconds: job.timeout().as_secs(),
        })
    }

    /// Create a delayed serialized job
    ///
    /// # Arguments
    ///
    /// * `job` - The job to serialize
    /// * `delay` - How long to delay execution
    pub fn from_job_delayed<J: Job>(job: J, delay: Duration) -> Result<Self, JobError> {
        let now = Utc::now();
        let delay_chrono = chrono::Duration::from_std(delay)
            .map_err(|e| JobError::Custom(format!("Invalid delay duration: {}", e)))?;

        Ok(Self {
            id: uuid::Uuid::new_v4(),
            job_type: std::any::type_name::<J>().to_string(),
            queue: job.queue().to_string(),
            payload: serde_json::to_value(&job).map_err(|e| JobError::SerializationError(e))?,
            attempts: 0,
            max_attempts: job.max_attempts(),
            backoff_seconds: job.backoff().as_secs(),
            queued_at: now,
            available_at: now + delay_chrono,
            timeout_seconds: job.timeout().as_secs(),
        })
    }

    /// Serialize to Redis-compatible string (JSON)
    ///
    /// # Errors
    ///
    /// Returns error if serialization fails (should not happen in practice)
    pub fn to_redis_payload(&self) -> String {
        serde_json::to_string(self).expect("SerializedJob should always serialize")
    }

    /// Deserialize from Redis string
    ///
    /// # Errors
    ///
    /// Returns `JobError::SerializationError` if the payload is not valid JSON
    /// or doesn't match the expected structure.
    pub fn from_redis_payload(s: &str) -> Result<Self, JobError> {
        serde_json::from_str(s).map_err(|e| JobError::SerializationError(e))
    }

    /// Check if job is ready to be processed
    pub fn is_available(&self) -> bool {
        Utc::now() >= self.available_at
    }

    /// Check if job has more retry attempts
    pub fn has_more_attempts(&self) -> bool {
        self.attempts < self.max_attempts
    }

    /// Increment attempt counter and update available_at for retry
    pub fn prepare_for_retry(&mut self) {
        self.attempts += 1;

        // Calculate exponential backoff
        let backoff_multiplier = 2u64.pow(self.attempts);
        let delay_seconds = self.backoff_seconds * backoff_multiplier;

        let delay = chrono::Duration::seconds(delay_seconds as i64);
        self.available_at = Utc::now() + delay;
    }

    /// Create a retry copy of this job with incremented attempt count
    pub fn create_retry(&self) -> Self {
        let mut retry = self.clone();
        retry.prepare_for_retry();
        retry
    }

    /// Get the job payload as a JSON string
    pub fn payload_str(&self) -> String {
        self.payload.to_string()
    }

    /// Calculate time until job is available
    pub fn time_until_available(&self) -> Option<Duration> {
        let now = Utc::now();
        if self.available_at > now {
            let duration = self.available_at - now;
            duration.to_std().ok()
        } else {
            None
        }
    }
}

/// Helper function to serialize a job for immediate dispatch
///
/// # Example
///
/// ```ignore
/// use rf_jobs::serialization::serialize_job;
/// use rf_jobs::Job;
///
/// # #[derive(serde::Serialize, serde::Deserialize, Clone)]
/// # struct MyJob;
/// # #[async_trait::async_trait]
/// # impl Job for MyJob {
/// #     async fn handle(&self, _: rf_jobs::JobContext) -> rf_jobs::JobResult { Ok(()) }
/// # }
/// let job = MyJob;
/// let serialized = serialize_job(job).unwrap();
/// let payload = serialized.to_redis_payload();
/// ```
pub fn serialize_job<J: Job>(job: J) -> Result<SerializedJob, JobError> {
    SerializedJob::from_job(job)
}

/// Helper function to serialize a delayed job
pub fn serialize_job_delayed<J: Job>(job: J, delay: Duration) -> Result<SerializedJob, JobError> {
    SerializedJob::from_job_delayed(job, delay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::JobContext;
    use async_trait::async_trait;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestJob {
        value: i32,
        name: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: JobContext) -> JobResult {
            Ok(())
        }

        fn max_attempts(&self) -> u32 {
            5
        }

        fn backoff(&self) -> Duration {
            Duration::from_secs(30)
        }
    }

    #[test]
    fn test_serialize_job() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job.clone()).unwrap();

        assert_eq!(serialized.queue, "default");
        assert_eq!(serialized.attempts, 0);
        assert_eq!(serialized.max_attempts, 5);
        assert_eq!(serialized.backoff_seconds, 30);
        assert!(serialized.is_available());
    }

    #[test]
    fn test_serialize_delayed_job() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let delay = Duration::from_secs(300);
        let serialized = SerializedJob::from_job_delayed(job, delay).unwrap();

        assert!(!serialized.is_available());
        assert!(serialized.time_until_available().is_some());
        assert!(serialized.time_until_available().unwrap().as_secs() > 290);
    }

    #[test]
    fn test_to_redis_payload() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job).unwrap();
        let payload = serialized.to_redis_payload();

        assert!(!payload.is_empty());
        assert!(payload.contains("\"value\":42"));
        assert!(payload.contains("\"name\":\"test\""));
    }

    #[test]
    fn test_from_redis_payload() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job.clone()).unwrap();
        let payload = serialized.to_redis_payload();

        let deserialized = SerializedJob::from_redis_payload(&payload).unwrap();

        assert_eq!(deserialized.id, serialized.id);
        assert_eq!(deserialized.queue, serialized.queue);
        assert_eq!(deserialized.attempts, serialized.attempts);
        assert_eq!(deserialized.max_attempts, serialized.max_attempts);
    }

    #[test]
    fn test_invalid_redis_payload() {
        let result = SerializedJob::from_redis_payload("invalid json");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            JobError::SerializationError(_)
        ));
    }

    #[test]
    fn test_has_more_attempts() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let mut serialized = SerializedJob::from_job(job).unwrap();
        serialized.max_attempts = 3;

        assert!(serialized.has_more_attempts()); // 0 < 3

        serialized.attempts = 2;
        assert!(serialized.has_more_attempts()); // 2 < 3

        serialized.attempts = 3;
        assert!(!serialized.has_more_attempts()); // 3 >= 3

        serialized.attempts = 4;
        assert!(!serialized.has_more_attempts()); // 4 >= 3
    }

    #[test]
    fn test_prepare_for_retry() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let mut serialized = SerializedJob::from_job(job).unwrap();
        serialized.backoff_seconds = 10;
        let original_available_at = serialized.available_at;

        serialized.prepare_for_retry();

        assert_eq!(serialized.attempts, 1);
        assert!(serialized.available_at > original_available_at);

        // Second retry should have longer backoff
        let first_retry_available_at = serialized.available_at;
        serialized.prepare_for_retry();

        assert_eq!(serialized.attempts, 2);
        assert!(serialized.available_at > first_retry_available_at);
    }

    #[test]
    fn test_create_retry() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job).unwrap();
        let original_attempts = serialized.attempts;

        let retry = serialized.create_retry();

        // Original should be unchanged
        assert_eq!(serialized.attempts, original_attempts);

        // Retry should have incremented attempts
        assert_eq!(retry.attempts, original_attempts + 1);
        assert_eq!(retry.id, serialized.id);
        assert_eq!(retry.payload, serialized.payload);
    }

    #[test]
    fn test_payload_str() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job).unwrap();
        let payload_str = serialized.payload_str();

        assert!(payload_str.contains("\"value\":42"));
        assert!(payload_str.contains("\"name\":\"test\""));
    }

    #[test]
    fn test_time_until_available_immediate() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let serialized = SerializedJob::from_job(job).unwrap();

        // Job should be available immediately
        assert!(serialized.time_until_available().is_none());
    }

    #[test]
    fn test_time_until_available_delayed() {
        let job = TestJob {
            value: 42,
            name: "test".to_string(),
        };

        let delay = Duration::from_secs(100);
        let serialized = SerializedJob::from_job_delayed(job, delay).unwrap();

        let time_until = serialized.time_until_available();
        assert!(time_until.is_some());
        assert!(time_until.unwrap().as_secs() > 90); // Allow some margin
        assert!(time_until.unwrap().as_secs() <= 100);
    }
}
