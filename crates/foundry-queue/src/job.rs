use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Job status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    /// Job is waiting to be processed
    Pending,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job is delayed and waiting for scheduled time
    Delayed,
    /// Job was cancelled
    Cancelled,
}

/// A job to be processed by the queue
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID
    pub id: String,
    /// Job name/type
    pub name: String,
    /// Job payload (serialized data)
    pub payload: Value,
    /// Number of attempts made
    pub attempts: u32,
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Delay before execution (in seconds)
    pub delay: Option<u64>,
    /// Job timeout (in seconds)
    pub timeout: Option<u64>,
    /// Queue name
    pub queue: String,
    /// Job priority (higher = more important)
    pub priority: i32,
    /// Job status
    pub status: JobStatus,
    /// When the job was created
    pub created_at: i64,
    /// When the job should be executed (for delayed jobs)
    pub execute_at: Option<i64>,
    /// When the job was last updated
    pub updated_at: i64,
    /// Custom metadata
    pub metadata: Value,
}

impl Job {
    /// Create a new job
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            payload: Value::Null,
            attempts: 0,
            max_attempts: 3,
            delay: None,
            timeout: None,
            queue: "default".to_string(),
            priority: 0,
            status: JobStatus::Pending,
            created_at: now,
            execute_at: None,
            updated_at: now,
            metadata: Value::Null,
        }
    }

    /// Set the job payload
    pub fn with_payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    /// Set the job delay
    pub fn with_delay(mut self, delay: Duration) -> Self {
        let delay_secs = delay.as_secs();
        self.delay = Some(delay_secs);
        self.execute_at = Some(self.created_at + delay_secs as i64);
        self.status = JobStatus::Delayed;
        self
    }

    /// Set the maximum number of retry attempts
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set the job timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout.as_secs());
        self
    }

    /// Set the queue name
    pub fn on_queue(mut self, queue: impl Into<String>) -> Self {
        self.queue = queue.into();
        self
    }

    /// Set the job priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        if self.metadata.is_null() {
            self.metadata = Value::Object(serde_json::Map::new());
        }
        if let Value::Object(ref mut obj) = self.metadata {
            obj.insert(key.into(), value);
        }
        self
    }

    /// Check if the job should be executed now
    pub fn should_execute(&self) -> bool {
        if let Some(execute_at) = self.execute_at {
            chrono::Utc::now().timestamp() >= execute_at
        } else {
            true
        }
    }

    /// Check if the job can be retried
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
    }

    /// Mark job as processing
    pub fn mark_processing(&mut self) {
        self.status = JobStatus::Processing;
        self.attempts += 1;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self) {
        self.status = JobStatus::Completed;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self) {
        self.status = JobStatus::Failed;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Convert to JSON bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Create from JSON bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Job execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Job ID
    pub job_id: String,
    /// Job name
    pub job_name: String,
    /// Success status
    pub success: bool,
    /// Result data
    pub data: Option<Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of attempts made
    pub attempts: u32,
}

impl JobResult {
    pub fn success(job: &Job, data: Option<Value>, execution_time_ms: u64) -> Self {
        Self {
            job_id: job.id.clone(),
            job_name: job.name.clone(),
            success: true,
            data,
            error: None,
            execution_time_ms,
            attempts: job.attempts,
        }
    }

    pub fn failure(job: &Job, error: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            job_id: job.id.clone(),
            job_name: job.name.clone(),
            success: false,
            data: None,
            error: Some(error.into()),
            execution_time_ms,
            attempts: job.attempts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_job_creation() {
        let job = Job::new("test");
        assert_eq!(job.name, "test");
        assert!(!job.id.is_empty());
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_job_with_delay() {
        let job = Job::new("test").with_delay(Duration::from_secs(60));
        assert_eq!(job.delay, Some(60));
        assert_eq!(job.status, JobStatus::Delayed);
        assert!(job.execute_at.is_some());
    }

    #[test]
    fn test_job_serialization() {
        let job = Job::new("test").with_payload(json!({"key": "value"}));
        let bytes = job.to_bytes().unwrap();
        let decoded = Job::from_bytes(&bytes).unwrap();
        assert_eq!(job.id, decoded.id);
        assert_eq!(job.name, decoded.name);
    }

    #[test]
    fn test_job_retry() {
        let mut job = Job::new("test").with_max_attempts(3);
        assert!(job.can_retry());

        job.mark_processing();
        assert_eq!(job.attempts, 1);
        assert!(job.can_retry());

        job.mark_processing();
        job.mark_processing();
        assert_eq!(job.attempts, 3);
        assert!(!job.can_retry());
    }
}
