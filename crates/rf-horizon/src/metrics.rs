//! Queue metrics and worker status tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Queue performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetrics {
    pub queue_name: String,
    pub jobs_processed: u64,
    pub jobs_failed: u64,
    pub jobs_pending: u64,
    pub average_wait_time_ms: f64,
    pub average_processing_time_ms: f64,
    pub throughput_per_minute: f64,
    pub updated_at: DateTime<Utc>,
}

impl QueueMetrics {
    /// Create new metrics for a queue
    pub fn new(queue_name: impl Into<String>) -> Self {
        Self {
            queue_name: queue_name.into(),
            jobs_processed: 0,
            jobs_failed: 0,
            jobs_pending: 0,
            average_wait_time_ms: 0.0,
            average_processing_time_ms: 0.0,
            throughput_per_minute: 0.0,
            updated_at: Utc::now(),
        }
    }

    /// Record a successful job
    pub fn record_success(&mut self, processing_time_ms: f64) {
        self.jobs_processed += 1;
        self.update_processing_time(processing_time_ms);
        self.calculate_throughput();
        self.updated_at = Utc::now();
    }

    /// Record a failed job
    pub fn record_failure(&mut self) {
        self.jobs_failed += 1;
        self.updated_at = Utc::now();
    }

    /// Update pending jobs count
    pub fn set_pending(&mut self, count: u64) {
        self.jobs_pending = count;
        self.updated_at = Utc::now();
    }

    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.jobs_processed + self.jobs_failed;
        if total == 0 {
            return 1.0;
        }
        self.jobs_processed as f64 / total as f64
    }

    fn update_processing_time(&mut self, new_time_ms: f64) {
        // Simple moving average
        let total_jobs = self.jobs_processed as f64;
        self.average_processing_time_ms =
            ((self.average_processing_time_ms * (total_jobs - 1.0)) + new_time_ms) / total_jobs;
    }

    fn calculate_throughput(&mut self) {
        // Simplified throughput calculation
        // In production, this would track jobs over a time window
        self.throughput_per_minute = self.jobs_processed as f64 / 60.0;
    }
}

/// Worker status information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkerStatus {
    Idle,
    Processing { job_name: String },
    Paused,
    Stopped,
}

/// Worker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: String,
    pub queue: String,
    pub status: WorkerStatus,
    pub jobs_processed: u64,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl WorkerInfo {
    /// Create a new worker info
    pub fn new(id: impl Into<String>, queue: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            queue: queue.into(),
            status: WorkerStatus::Idle,
            jobs_processed: 0,
            started_at: Utc::now(),
            last_activity: Utc::now(),
        }
    }

    /// Mark worker as processing a job
    pub fn start_processing(&mut self, job_name: impl Into<String>) {
        self.status = WorkerStatus::Processing {
            job_name: job_name.into(),
        };
        self.last_activity = Utc::now();
    }

    /// Mark worker as idle
    pub fn finish_processing(&mut self) {
        self.status = WorkerStatus::Idle;
        self.jobs_processed += 1;
        self.last_activity = Utc::now();
    }

    /// Check if worker is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkerStatus::Idle | WorkerStatus::Processing { .. }
        )
    }
}

/// Job history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHistoryEntry {
    pub id: String,
    pub queue: String,
    pub job_name: String,
    pub status: JobHistoryStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Job history status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobHistoryStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl JobHistoryEntry {
    /// Create a new job history entry
    pub fn new(
        id: impl Into<String>,
        queue: impl Into<String>,
        job_name: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            queue: queue.into(),
            job_name: job_name.into(),
            status: JobHistoryStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            error: None,
        }
    }

    /// Mark job as completed
    pub fn complete(&mut self) {
        self.status = JobHistoryStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.duration_ms =
            Some((self.completed_at.unwrap() - self.started_at).num_milliseconds() as u64);
    }

    /// Mark job as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobHistoryStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
        self.duration_ms =
            Some((self.completed_at.unwrap() - self.started_at).num_milliseconds() as u64);
    }
}
