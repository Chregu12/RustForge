//! Job watcher for monitoring queued jobs

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// Job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub job_name: String,
    pub queue: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl JobInfo {
    /// Create a new job info
    pub fn new(job_name: impl Into<String>, queue: impl Into<String>) -> Self {
        Self {
            job_name: job_name.into(),
            queue: queue.into(),
            payload: json!({}),
            status: JobStatus::Pending,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error: None,
            created_at: Utc::now(),
        }
    }

    /// Set payload
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Mark job as processing
    pub fn processing(mut self) -> Self {
        self.status = JobStatus::Processing;
        self.started_at = Some(Utc::now());
        self
    }

    /// Mark job as completed
    pub fn completed(mut self) -> Self {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());

        if let Some(started) = self.started_at {
            self.duration_ms =
                Some((self.completed_at.unwrap() - started).num_milliseconds() as u64);
        }

        self
    }

    /// Mark job as failed
    pub fn failed(mut self, error: impl Into<String>) -> Self {
        self.status = JobStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());

        if let Some(started) = self.started_at {
            self.duration_ms =
                Some((self.completed_at.unwrap() - started).num_milliseconds() as u64);
        }

        self
    }
}

/// Job watcher for monitoring queued jobs
#[derive(Clone)]
pub struct JobWatcher {
    storage: Storage,
}

impl JobWatcher {
    /// Create a new job watcher
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Record a job
    pub async fn record(&self, info: JobInfo) {
        let entry = Entry::new(
            EntryType::Job,
            json!({
                "job_name": info.job_name,
                "queue": info.queue,
                "payload": info.payload,
                "status": info.status,
                "started_at": info.started_at,
                "completed_at": info.completed_at,
                "duration_ms": info.duration_ms,
                "error": info.error,
                "created_at": info.created_at,
            }),
        )
        .with_tag(format!("queue:{}", info.queue))
        .with_tag(format!("status:{:?}", info.status).to_lowercase());

        self.storage.store(entry).await;
    }

    /// Get all recorded jobs
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Job).await
    }

    /// Get jobs by queue
    pub async fn by_queue(&self, queue: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("queue:{}", queue);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }

    /// Get failed jobs
    pub async fn failed_jobs(&self) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| entry.tags.contains(&"status:failed".to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_info_creation() {
        let info =
            JobInfo::new("SendEmailJob", "emails").with_payload(json!({"to": "user@example.com"}));

        assert_eq!(info.job_name, "SendEmailJob");
        assert_eq!(info.queue, "emails");
        assert_eq!(info.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_job_watcher_record() {
        let storage = Storage::new();
        let watcher = JobWatcher::new(storage);

        let info = JobInfo::new("ProcessPayment", "payments")
            .processing()
            .completed();

        watcher.record(info).await;

        let jobs = watcher.all().await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].content["job_name"], "ProcessPayment");
    }

    #[tokio::test]
    async fn test_job_by_queue() {
        let storage = Storage::new();
        let watcher = JobWatcher::new(storage);

        watcher.record(JobInfo::new("Job1", "emails")).await;
        watcher.record(JobInfo::new("Job2", "payments")).await;
        watcher.record(JobInfo::new("Job3", "emails")).await;

        let email_jobs = watcher.by_queue("emails").await;
        assert_eq!(email_jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_failed_jobs() {
        let storage = Storage::new();
        let watcher = JobWatcher::new(storage);

        watcher
            .record(JobInfo::new("Job1", "default").processing().completed())
            .await;
        watcher
            .record(
                JobInfo::new("Job2", "default")
                    .processing()
                    .failed("Connection timeout"),
            )
            .await;
        watcher
            .record(
                JobInfo::new("Job3", "default")
                    .processing()
                    .failed("Invalid data"),
            )
            .await;

        let failed = watcher.failed_jobs().await;
        assert_eq!(failed.len(), 2);
    }
}
