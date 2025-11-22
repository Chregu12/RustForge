//! Failed job handling with retry and pruning capabilities

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Information about a failed job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedJob {
    pub id: String,
    pub queue: String,
    pub job_name: String,
    pub payload: String,
    pub exception: String,
    pub failed_at: DateTime<Utc>,
    pub retry_count: u32,
}

impl FailedJob {
    /// Create a new failed job record
    pub fn new(
        queue: impl Into<String>,
        job_name: impl Into<String>,
        payload: impl Into<String>,
        exception: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            queue: queue.into(),
            job_name: job_name.into(),
            payload: payload.into(),
            exception: exception.into(),
            failed_at: Utc::now(),
            retry_count: 0,
        }
    }

    /// Check if job is eligible for retry based on age
    pub fn can_retry(&self, max_retry_count: u32) -> bool {
        self.retry_count < max_retry_count
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// Retry strategy for failed jobs
#[derive(Debug, Clone)]
pub enum RetryStrategy {
    /// Immediate retry
    Immediate,
    /// Linear backoff (retry_count * base_delay_seconds)
    Linear { base_delay_seconds: u64 },
    /// Exponential backoff (base_delay_seconds * 2^retry_count)
    Exponential { base_delay_seconds: u64 },
}

impl RetryStrategy {
    /// Calculate delay before next retry
    pub fn delay(&self, retry_count: u32) -> Duration {
        match self {
            RetryStrategy::Immediate => Duration::seconds(0),
            RetryStrategy::Linear { base_delay_seconds } => {
                Duration::seconds((retry_count as i64) * (*base_delay_seconds as i64))
            }
            RetryStrategy::Exponential { base_delay_seconds } => {
                let multiplier = 2_u64.pow(retry_count);
                Duration::seconds((*base_delay_seconds as i64) * (multiplier as i64))
            }
        }
    }
}

/// Handler for managing failed jobs
pub struct FailedJobHandler {
    failed_jobs: Arc<RwLock<HashMap<String, FailedJob>>>,
    retry_strategy: RetryStrategy,
    max_retry_count: u32,
}

impl FailedJobHandler {
    /// Create a new failed job handler
    pub fn new() -> Self {
        Self {
            failed_jobs: Arc::new(RwLock::new(HashMap::new())),
            retry_strategy: RetryStrategy::Exponential {
                base_delay_seconds: 60,
            },
            max_retry_count: 3,
        }
    }

    /// Set retry strategy
    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }

    /// Set maximum retry count
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retry_count = max_retries;
        self
    }

    /// Record a failed job
    pub async fn record(&self, failed_job: FailedJob) {
        let mut jobs = self.failed_jobs.write().await;
        jobs.insert(failed_job.id.clone(), failed_job);
    }

    /// Get all failed jobs
    pub async fn all(&self) -> Vec<FailedJob> {
        let jobs = self.failed_jobs.read().await;
        jobs.values().cloned().collect()
    }

    /// Get failed jobs for a specific queue
    pub async fn for_queue(&self, queue_name: &str) -> Vec<FailedJob> {
        let jobs = self.failed_jobs.read().await;
        jobs.values()
            .filter(|job| job.queue == queue_name)
            .cloned()
            .collect()
    }

    /// Get a specific failed job by ID
    pub async fn find(&self, job_id: &str) -> Option<FailedJob> {
        let jobs = self.failed_jobs.read().await;
        jobs.get(job_id).cloned()
    }

    /// Retry a specific failed job
    pub async fn retry(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.failed_jobs.write().await;

        if let Some(job) = jobs.get_mut(job_id) {
            if !job.can_retry(self.max_retry_count) {
                anyhow::bail!(
                    "Job has exceeded maximum retry count of {}",
                    self.max_retry_count
                );
            }

            job.increment_retry();

            // Calculate delay based on strategy
            let delay = self.retry_strategy.delay(job.retry_count);

            // In a real implementation, this would re-queue the job
            // For now, we'll just log it
            println!(
                "Retrying job {} (attempt {}) after {} seconds",
                job.id,
                job.retry_count,
                delay.num_seconds()
            );

            Ok(())
        } else {
            anyhow::bail!("Failed job with ID {} not found", job_id)
        }
    }

    /// Retry all failed jobs for a queue
    pub async fn retry_all(&self, queue_name: &str) -> Result<usize> {
        let job_ids: Vec<String> = {
            let jobs = self.failed_jobs.read().await;
            jobs.values()
                .filter(|job| job.queue == queue_name)
                .map(|job| job.id.clone())
                .collect()
        };

        let mut retried = 0;
        for job_id in job_ids {
            if self.retry(job_id.as_str()).await.is_ok() {
                retried += 1;
            }
        }

        Ok(retried)
    }

    /// Delete a specific failed job
    pub async fn delete(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.failed_jobs.write().await;

        if jobs.remove(job_id).is_some() {
            Ok(())
        } else {
            anyhow::bail!("Failed job with ID {} not found", job_id)
        }
    }

    /// Prune failed jobs older than specified days
    pub async fn prune(&self, older_than_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - Duration::days(older_than_days);
        let mut jobs = self.failed_jobs.write().await;

        let to_remove: Vec<String> = jobs
            .values()
            .filter(|job| job.failed_at < cutoff)
            .map(|job| job.id.clone())
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            jobs.remove(&id);
        }

        Ok(count)
    }

    /// Get count of failed jobs
    pub async fn count(&self) -> usize {
        self.failed_jobs.read().await.len()
    }

    /// Clear all failed jobs
    pub async fn clear(&self) {
        let mut jobs = self.failed_jobs.write().await;
        jobs.clear();
    }
}

impl Default for FailedJobHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_failed_job_creation() {
        let job = FailedJob::new("default", "TestJob", "{}", "Error occurred");
        assert_eq!(job.queue, "default");
        assert_eq!(job.job_name, "TestJob");
        assert_eq!(job.retry_count, 0);
    }

    #[tokio::test]
    async fn test_failed_job_handler() {
        let handler = FailedJobHandler::new();
        let job = FailedJob::new("emails", "SendEmailJob", "{}", "SMTP error");

        handler.record(job.clone()).await;

        let all_jobs = handler.all().await;
        assert_eq!(all_jobs.len(), 1);
        assert_eq!(all_jobs[0].queue, "emails");
    }

    #[tokio::test]
    async fn test_filter_by_queue() {
        let handler = FailedJobHandler::new();

        handler
            .record(FailedJob::new("emails", "Job1", "{}", "Error"))
            .await;
        handler
            .record(FailedJob::new("default", "Job2", "{}", "Error"))
            .await;
        handler
            .record(FailedJob::new("emails", "Job3", "{}", "Error"))
            .await;

        let email_jobs = handler.for_queue("emails").await;
        assert_eq!(email_jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_retry_job() {
        let handler = FailedJobHandler::new();
        let job = FailedJob::new("default", "TestJob", "{}", "Error");
        let job_id = job.id.clone();

        handler.record(job).await;

        handler.retry(&job_id).await.unwrap();

        let job = handler.find(&job_id).await.unwrap();
        assert_eq!(job.retry_count, 1);
    }

    #[tokio::test]
    async fn test_retry_all() {
        let handler = FailedJobHandler::new();

        handler
            .record(FailedJob::new("emails", "Job1", "{}", "Error"))
            .await;
        handler
            .record(FailedJob::new("emails", "Job2", "{}", "Error"))
            .await;
        handler
            .record(FailedJob::new("default", "Job3", "{}", "Error"))
            .await;

        let retried = handler.retry_all("emails").await.unwrap();
        assert_eq!(retried, 2);
    }

    #[tokio::test]
    async fn test_delete_job() {
        let handler = FailedJobHandler::new();
        let job = FailedJob::new("default", "TestJob", "{}", "Error");
        let job_id = job.id.clone();

        handler.record(job).await;
        assert_eq!(handler.count().await, 1);

        handler.delete(&job_id).await.unwrap();
        assert_eq!(handler.count().await, 0);
    }

    #[tokio::test]
    async fn test_retry_strategy_immediate() {
        let strategy = RetryStrategy::Immediate;
        assert_eq!(strategy.delay(0).num_seconds(), 0);
        assert_eq!(strategy.delay(5).num_seconds(), 0);
    }

    #[tokio::test]
    async fn test_retry_strategy_linear() {
        let strategy = RetryStrategy::Linear {
            base_delay_seconds: 10,
        };
        assert_eq!(strategy.delay(0).num_seconds(), 0);
        assert_eq!(strategy.delay(1).num_seconds(), 10);
        assert_eq!(strategy.delay(2).num_seconds(), 20);
    }

    #[tokio::test]
    async fn test_retry_strategy_exponential() {
        let strategy = RetryStrategy::Exponential {
            base_delay_seconds: 5,
        };
        assert_eq!(strategy.delay(0).num_seconds(), 5);
        assert_eq!(strategy.delay(1).num_seconds(), 10);
        assert_eq!(strategy.delay(2).num_seconds(), 20);
        assert_eq!(strategy.delay(3).num_seconds(), 40);
    }

    #[tokio::test]
    async fn test_prune_old_jobs() {
        let handler = FailedJobHandler::new();

        // Create an old job
        let mut old_job = FailedJob::new("default", "OldJob", "{}", "Error");
        old_job.failed_at = Utc::now() - Duration::days(40);
        handler.record(old_job).await;

        // Create a recent job
        handler
            .record(FailedJob::new("default", "NewJob", "{}", "Error"))
            .await;

        assert_eq!(handler.count().await, 2);

        // Prune jobs older than 30 days
        let pruned = handler.prune(30).await.unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(handler.count().await, 1);
    }
}
