//! Recent jobs store for tracking job history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tags::JobTags;

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Reserved,
    Completed,
    Failed,
    Retrying,
}

/// Recent job entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentJob {
    pub id: String,
    pub name: String,
    pub queue: String,
    pub status: JobStatus,
    pub tags: Vec<String>,
    pub payload: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub runtime: Option<f64>,     // Seconds
    pub attempt: u32,
    pub max_attempts: u32,
    pub error: Option<String>,
}

impl RecentJob {
    /// Create a new recent job
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        queue: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            queue: queue.into(),
            status: JobStatus::Pending,
            tags: Vec::new(),
            payload: serde_json::Value::Null,
            started_at: Utc::now(),
            completed_at: None,
            runtime: None,
            attempt: 0,
            max_attempts: 3,
            error: None,
        }
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set payload
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Set attempt
    pub fn with_attempt(mut self, attempt: u32, max_attempts: u32) -> Self {
        self.attempt = attempt;
        self.max_attempts = max_attempts;
        self
    }

    /// Mark as reserved
    pub fn reserve(&mut self) {
        self.status = JobStatus::Reserved;
    }

    /// Mark as completed
    pub fn complete(&mut self) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.calculate_runtime();
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = JobStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
        self.calculate_runtime();
    }

    /// Mark as retrying
    pub fn retry(&mut self) {
        self.status = JobStatus::Retrying;
        self.attempt += 1;
    }

    /// Calculate runtime
    fn calculate_runtime(&mut self) {
        if let Some(completed) = self.completed_at {
            self.runtime = Some((completed - self.started_at).num_milliseconds() as f64 / 1000.0);
        }
    }

    /// Check if job has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if job is still pending or reserved
    pub fn is_pending(&self) -> bool {
        matches!(self.status, JobStatus::Pending | JobStatus::Reserved)
    }

    /// Check if job is completed
    pub fn is_completed(&self) -> bool {
        self.status == JobStatus::Completed
    }

    /// Check if job is failed
    pub fn is_failed(&self) -> bool {
        self.status == JobStatus::Failed
    }
}

/// Store for recent jobs
pub struct RecentJobsStore {
    jobs: Arc<RwLock<VecDeque<RecentJob>>>,
    max_recent: usize,
}

impl RecentJobsStore {
    /// Create a new recent jobs store
    pub fn new(max_recent: usize) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(VecDeque::with_capacity(max_recent))),
            max_recent,
        }
    }

    /// Create with default capacity (10,000 jobs)
    pub fn with_default_capacity() -> Self {
        Self::new(10_000)
    }

    /// Create with small capacity (1,000 jobs)
    pub fn with_small_capacity() -> Self {
        Self::new(1_000)
    }

    /// Create with large capacity (100,000 jobs)
    pub fn with_large_capacity() -> Self {
        Self::new(100_000)
    }

    /// Add a job
    pub async fn add(&self, job: RecentJob) {
        let mut jobs = self.jobs.write().await;
        jobs.push_back(job);

        // Remove oldest if we exceed capacity
        while jobs.len() > self.max_recent {
            jobs.pop_front();
        }
    }

    /// Get all jobs
    pub async fn all(&self) -> Vec<RecentJob> {
        self.jobs.read().await.iter().cloned().collect()
    }

    /// Get jobs by status
    pub async fn by_status(&self, status: JobStatus) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.status == status)
            .cloned()
            .collect()
    }

    /// Get pending jobs
    pub async fn pending(&self) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.is_pending())
            .cloned()
            .collect()
    }

    /// Get completed jobs
    pub async fn completed(&self) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.is_completed())
            .cloned()
            .collect()
    }

    /// Get failed jobs
    pub async fn failed(&self) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.is_failed())
            .cloned()
            .collect()
    }

    /// Get jobs by queue
    pub async fn by_queue(&self, queue: &str) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.queue == queue)
            .cloned()
            .collect()
    }

    /// Get jobs by tag
    pub async fn by_tag(&self, tag: &str) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.has_tag(tag))
            .cloned()
            .collect()
    }

    /// Get a specific job by ID
    pub async fn get(&self, job_id: &str) -> Option<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .find(|j| j.id == job_id)
            .cloned()
    }

    /// Get latest N jobs
    pub async fn latest(&self, n: usize) -> Vec<RecentJob> {
        let jobs = self.jobs.read().await;
        let start = if jobs.len() > n { jobs.len() - n } else { 0 };
        jobs.range(start..).cloned().collect()
    }

    /// Get oldest N jobs
    pub async fn oldest(&self, n: usize) -> Vec<RecentJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().take(n).cloned().collect()
    }

    /// Get jobs within time range
    pub async fn range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<RecentJob> {
        self.jobs
            .read()
            .await
            .iter()
            .filter(|j| j.started_at >= from && j.started_at <= to)
            .cloned()
            .collect()
    }

    /// Get jobs from last N minutes
    pub async fn last_minutes(&self, minutes: i64) -> Vec<RecentJob> {
        let from = Utc::now() - chrono::Duration::minutes(minutes);
        let to = Utc::now();
        self.range(from, to).await
    }

    /// Get jobs from last N hours
    pub async fn last_hours(&self, hours: i64) -> Vec<RecentJob> {
        let from = Utc::now() - chrono::Duration::hours(hours);
        let to = Utc::now();
        self.range(from, to).await
    }

    /// Update a job's status
    pub async fn update_status(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = status;
        }
    }

    /// Mark a job as completed
    pub async fn complete_job(&self, job_id: &str) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.complete();
        }
    }

    /// Mark a job as failed
    pub async fn fail_job(&self, job_id: &str, error: impl Into<String>) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.fail(error);
        }
    }

    /// Get job count
    pub async fn count(&self) -> usize {
        self.jobs.read().await.len()
    }

    /// Get statistics
    pub async fn stats(&self) -> RecentJobsStats {
        let jobs = self.jobs.read().await;

        let total = jobs.len();
        let pending = jobs.iter().filter(|j| j.is_pending()).count();
        let completed = jobs.iter().filter(|j| j.is_completed()).count();
        let failed = jobs.iter().filter(|j| j.is_failed()).count();

        let avg_runtime = if completed > 0 {
            let total_runtime: f64 = jobs
                .iter()
                .filter(|j| j.is_completed())
                .filter_map(|j| j.runtime)
                .sum();
            total_runtime / completed as f64
        } else {
            0.0
        };

        let success_rate = if completed + failed > 0 {
            completed as f64 / (completed + failed) as f64
        } else {
            1.0
        };

        RecentJobsStats {
            total,
            pending,
            completed,
            failed,
            avg_runtime,
            success_rate,
        }
    }

    /// Clear all jobs
    pub async fn clear(&self) {
        self.jobs.write().await.clear();
    }
}

impl Clone for RecentJobsStore {
    fn clone(&self) -> Self {
        Self {
            jobs: Arc::clone(&self.jobs),
            max_recent: self.max_recent,
        }
    }
}

impl Default for RecentJobsStore {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

/// Statistics for recent jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentJobsStats {
    pub total: usize,
    pub pending: usize,
    pub completed: usize,
    pub failed: usize,
    pub avg_runtime: f64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recent_job_new() {
        let job = RecentJob::new("job-123", "SendEmail", "emails");
        assert_eq!(job.id, "job-123");
        assert_eq!(job.name, "SendEmail");
        assert_eq!(job.queue, "emails");
        assert_eq!(job.status, JobStatus::Pending);
    }

    #[test]
    fn test_recent_job_with_tags() {
        let job = RecentJob::new("job-123", "SendEmail", "emails")
            .with_tags(vec!["user:123".to_string(), "priority:high".to_string()]);

        assert_eq!(job.tags.len(), 2);
        assert!(job.has_tag("user:123"));
    }

    #[test]
    fn test_recent_job_complete() {
        let mut job = RecentJob::new("job-123", "SendEmail", "emails");
        job.complete();

        assert!(job.is_completed());
        assert!(job.completed_at.is_some());
        assert!(job.runtime.is_some());
    }

    #[test]
    fn test_recent_job_fail() {
        let mut job = RecentJob::new("job-123", "SendEmail", "emails");
        job.fail("SMTP error");

        assert!(job.is_failed());
        assert_eq!(job.error, Some("SMTP error".to_string()));
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_recent_job_retry() {
        let mut job = RecentJob::new("job-123", "SendEmail", "emails");
        job.retry();

        assert_eq!(job.status, JobStatus::Retrying);
        assert_eq!(job.attempt, 1);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_add() {
        let store = RecentJobsStore::new(100);
        let job = RecentJob::new("job-123", "SendEmail", "emails");

        store.add(job).await;

        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_max_capacity() {
        let store = RecentJobsStore::new(3);

        for i in 0..5 {
            let job = RecentJob::new(format!("job-{}", i), "SendEmail", "emails");
            store.add(job).await;
        }

        // Should only keep last 3 jobs
        assert_eq!(store.count().await, 3);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_by_status() {
        let store = RecentJobsStore::new(100);

        let mut job1 = RecentJob::new("job-1", "SendEmail", "emails");
        job1.complete();

        let mut job2 = RecentJob::new("job-2", "SendEmail", "emails");
        job2.fail("Error");

        let job3 = RecentJob::new("job-3", "SendEmail", "emails");

        store.add(job1).await;
        store.add(job2).await;
        store.add(job3).await;

        let completed = store.completed().await;
        assert_eq!(completed.len(), 1);

        let failed = store.failed().await;
        assert_eq!(failed.len(), 1);

        let pending = store.pending().await;
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_by_queue() {
        let store = RecentJobsStore::new(100);

        store.add(RecentJob::new("job-1", "SendEmail", "emails")).await;
        store.add(RecentJob::new("job-2", "ProcessOrder", "default")).await;
        store.add(RecentJob::new("job-3", "SendEmail", "emails")).await;

        let email_jobs = store.by_queue("emails").await;
        assert_eq!(email_jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_by_tag() {
        let store = RecentJobsStore::new(100);

        store.add(
            RecentJob::new("job-1", "SendEmail", "emails")
                .with_tag("user:123")
        ).await;
        store.add(
            RecentJob::new("job-2", "SendEmail", "emails")
                .with_tag("user:456")
        ).await;
        store.add(
            RecentJob::new("job-3", "SendEmail", "emails")
                .with_tag("user:123")
        ).await;

        let jobs = store.by_tag("user:123").await;
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_latest() {
        let store = RecentJobsStore::new(100);

        for i in 0..10 {
            store.add(RecentJob::new(format!("job-{}", i), "SendEmail", "emails")).await;
        }

        let latest = store.latest(3).await;
        assert_eq!(latest.len(), 3);
        assert_eq!(latest[0].id, "job-7");
        assert_eq!(latest[2].id, "job-9");
    }

    #[tokio::test]
    async fn test_recent_jobs_store_stats() {
        let store = RecentJobsStore::new(100);

        let mut job1 = RecentJob::new("job-1", "SendEmail", "emails");
        job1.complete();

        let mut job2 = RecentJob::new("job-2", "SendEmail", "emails");
        job2.complete();

        let mut job3 = RecentJob::new("job-3", "SendEmail", "emails");
        job3.fail("Error");

        store.add(job1).await;
        store.add(job2).await;
        store.add(job3).await;

        let stats = store.stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 2);
        assert_eq!(stats.failed, 1);
        assert!((stats.success_rate - 0.666).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_recent_jobs_store_complete_job() {
        let store = RecentJobsStore::new(100);
        let job = RecentJob::new("job-123", "SendEmail", "emails");

        store.add(job).await;
        store.complete_job("job-123").await;

        let job = store.get("job-123").await.unwrap();
        assert!(job.is_completed());
    }
}
