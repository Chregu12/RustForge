//! Queue fake implementation for testing
//!
//! Provides a fake Queue implementation that records all dispatched jobs
//! and allows assertions on what was pushed.

use serde::de::DeserializeOwned;
use std::sync::{Arc, Mutex};

/// Record of a pushed job
#[derive(Debug, Clone)]
pub struct JobRecord {
    /// Job type identifier
    pub job_type: String,

    /// Serialized job payload
    pub payload: serde_json::Value,

    /// Queue name
    pub queue: String,

    /// Job ID
    pub job_id: String,

    /// Job priority
    pub priority: i32,
}

/// Queue fake for testing
///
/// Records all jobs that are pushed to the queue and provides
/// assertion methods to verify behavior.
///
/// # Example
///
/// ```ignore
/// use rf_testing::fakes::QueueFake;
///
/// let fake = QueueFake::new();
///
/// // Push some jobs
/// fake.push(job_metadata).await?;
///
/// // Assert
/// fake.assert_pushed("send_email");
/// fake.assert_pushed_times("send_email", 1);
/// fake.assert_pushed_on("send_email", "default");
/// ```
#[derive(Clone)]
pub struct QueueFake {
    records: Arc<Mutex<Vec<JobRecord>>>,
}

impl QueueFake {
    /// Create a new queue fake
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all pushed jobs
    pub fn pushed_jobs(&self) -> Vec<JobRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Get pushed jobs of a specific type
    pub fn pushed_jobs_of_type(&self, job_type: &str) -> Vec<JobRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.job_type == job_type)
            .cloned()
            .collect()
    }

    /// Assert that a job of the given type was pushed
    ///
    /// # Panics
    ///
    /// Panics if no job of the given type was pushed.
    pub fn assert_pushed(&self, job_type: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| r.job_type == job_type) {
            panic!(
                "Failed asserting that job '{}' was pushed. Pushed jobs: {:?}",
                job_type,
                records.iter().map(|r| &r.job_type).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a job of the given type was pushed exactly N times
    ///
    /// # Panics
    ///
    /// Panics if the job was not pushed exactly N times.
    pub fn assert_pushed_times(&self, job_type: &str, times: usize) {
        let records = self.records.lock().unwrap();
        let count = records.iter().filter(|r| r.job_type == job_type).count();

        if count != times {
            panic!(
                "Failed asserting that job '{}' was pushed {} times. Actually pushed {} times.",
                job_type, times, count
            );
        }
    }

    /// Assert that a job was pushed to a specific queue
    ///
    /// # Panics
    ///
    /// Panics if the job was not pushed to the specified queue.
    pub fn assert_pushed_on(&self, job_type: &str, queue: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| r.job_type == job_type && r.queue == queue) {
            panic!(
                "Failed asserting that job '{}' was pushed on queue '{}'. Found on queues: {:?}",
                job_type,
                queue,
                records
                    .iter()
                    .filter(|r| r.job_type == job_type)
                    .map(|r| &r.queue)
                    .collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a job of the given type was NOT pushed
    ///
    /// # Panics
    ///
    /// Panics if the job was pushed.
    pub fn assert_not_pushed(&self, job_type: &str) {
        let records = self.records.lock().unwrap();

        if records.iter().any(|r| r.job_type == job_type) {
            panic!("Failed asserting that job '{}' was not pushed", job_type);
        }
    }

    /// Assert that no jobs were pushed at all
    ///
    /// # Panics
    ///
    /// Panics if any jobs were pushed.
    pub fn assert_nothing_pushed(&self) {
        let records = self.records.lock().unwrap();

        if !records.is_empty() {
            panic!(
                "Failed asserting that no jobs were pushed. {} jobs were pushed: {:?}",
                records.len(),
                records.iter().map(|r| &r.job_type).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a job was pushed with specific payload values
    ///
    /// Uses a closure to inspect the job payload.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fake.assert_pushed_with::<SendEmailJob>("send_email", |job| {
    ///     job.to == "test@example.com"
    /// });
    /// ```
    pub fn assert_pushed_with<F>(&self, job_type: &str, predicate: F)
    where
        F: Fn(&serde_json::Value) -> bool,
    {
        let records = self.records.lock().unwrap();

        let found = records
            .iter()
            .filter(|r| r.job_type == job_type)
            .any(|r| predicate(&r.payload));

        if !found {
            panic!(
                "Failed asserting that job '{}' was pushed with matching payload",
                job_type
            );
        }
    }

    /// Get the first pushed job of a specific type and deserialize it
    ///
    /// Returns None if no job of that type was pushed.
    pub fn first_pushed<T>(&self, job_type: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let records = self.records.lock().unwrap();

        records
            .iter()
            .find(|r| r.job_type == job_type)
            .and_then(|r| serde_json::from_value(r.payload.clone()).ok())
    }

    /// Get all pushed jobs of a specific type and deserialize them
    pub fn all_pushed<T>(&self, job_type: &str) -> Vec<T>
    where
        T: DeserializeOwned,
    {
        let records = self.records.lock().unwrap();

        records
            .iter()
            .filter(|r| r.job_type == job_type)
            .filter_map(|r| serde_json::from_value(r.payload.clone()).ok())
            .collect()
    }

    /// Clear all recorded jobs
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    /// Get the total number of pushed jobs
    pub fn count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// Get the number of pushed jobs of a specific type
    pub fn count_of_type(&self, job_type: &str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.job_type == job_type)
            .count()
    }

    /// Record a job push manually (for testing)
    ///
    /// This is primarily used for testing the QueueFake itself.
    /// In normal usage, the Queue trait implementation would call this internally.
    pub fn record_push(&self, record: JobRecord) {
        self.records.lock().unwrap().push(record);
    }
}

impl Default for QueueFake {
    fn default() -> Self {
        Self::new()
    }
}

// Note: The actual Queue trait implementation would be in the integration
// with rf-queue. This fake is designed to be used standalone in tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_fake_creation() {
        let fake = QueueFake::new();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_record_and_retrieve() {
        let fake = QueueFake::new();

        let record = JobRecord {
            job_type: "test_job".to_string(),
            payload: serde_json::json!({"test": "data"}),
            queue: "default".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        };

        fake.record_push(record);

        assert_eq!(fake.count(), 1);
        assert_eq!(fake.count_of_type("test_job"), 1);
    }

    #[test]
    fn test_assert_pushed() {
        let fake = QueueFake::new();

        let record = JobRecord {
            job_type: "test_job".to_string(),
            payload: serde_json::json!({}),
            queue: "default".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        };

        fake.record_push(record);
        fake.assert_pushed("test_job");
    }

    #[test]
    #[should_panic(expected = "Failed asserting that job 'missing_job' was pushed")]
    fn test_assert_pushed_fails() {
        let fake = QueueFake::new();
        fake.assert_pushed("missing_job");
    }

    #[test]
    fn test_assert_pushed_times() {
        let fake = QueueFake::new();

        for i in 0..3 {
            fake.record_push(JobRecord {
                job_type: "test_job".to_string(),
                payload: serde_json::json!({}),
                queue: "default".to_string(),
                job_id: i.to_string(),
                priority: 0,
            });
        }

        fake.assert_pushed_times("test_job", 3);
    }

    #[test]
    #[should_panic(expected = "Failed asserting that job 'test_job' was pushed 5 times")]
    fn test_assert_pushed_times_fails() {
        let fake = QueueFake::new();
        fake.record_push(JobRecord {
            job_type: "test_job".to_string(),
            payload: serde_json::json!({}),
            queue: "default".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        });

        fake.assert_pushed_times("test_job", 5);
    }

    #[test]
    fn test_assert_pushed_on() {
        let fake = QueueFake::new();

        fake.record_push(JobRecord {
            job_type: "test_job".to_string(),
            payload: serde_json::json!({}),
            queue: "emails".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        });

        fake.assert_pushed_on("test_job", "emails");
    }

    #[test]
    fn test_assert_not_pushed() {
        let fake = QueueFake::new();
        fake.assert_not_pushed("test_job");
    }

    #[test]
    fn test_assert_nothing_pushed() {
        let fake = QueueFake::new();
        fake.assert_nothing_pushed();
    }

    #[test]
    fn test_clear() {
        let fake = QueueFake::new();

        fake.record_push(JobRecord {
            job_type: "test_job".to_string(),
            payload: serde_json::json!({}),
            queue: "default".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        });

        assert_eq!(fake.count(), 1);
        fake.clear();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_assert_pushed_with() {
        let fake = QueueFake::new();

        fake.record_push(JobRecord {
            job_type: "send_email".to_string(),
            payload: serde_json::json!({
                "to": "test@example.com",
                "subject": "Hello"
            }),
            queue: "default".to_string(),
            job_id: "123".to_string(),
            priority: 0,
        });

        fake.assert_pushed_with("send_email", |payload| {
            payload["to"] == "test@example.com"
        });
    }
}
