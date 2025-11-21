//! Job batching with progress tracking

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use async_trait::async_trait;

/// Job trait for batch execution
#[async_trait]
pub trait Job: Send + Sync {
    async fn handle(&self) -> Result<()>;
    fn name(&self) -> String;
}

/// Batch status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// Batch progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    pub batch_id: String,
    pub name: String,
    pub total_jobs: usize,
    pub pending_jobs: usize,
    pub failed_jobs: usize,
    pub status: BatchStatus,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl BatchProgress {
    /// Calculate progress percentage (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_jobs == 0 {
            return 1.0;
        }
        let completed = self.total_jobs - self.pending_jobs;
        completed as f64 / self.total_jobs as f64
    }

    /// Check if batch is finished
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            BatchStatus::Completed | BatchStatus::Failed | BatchStatus::Cancelled
        )
    }
}

type ThenCallback = Arc<dyn Fn(&BatchProgress) + Send + Sync>;
type CatchCallback = Arc<dyn Fn(&BatchProgress, &str) + Send + Sync>;

/// Job batch for managing multiple related jobs
pub struct Batch {
    id: String,
    name: String,
    jobs: Vec<Arc<dyn Job>>,
    progress: Arc<RwLock<BatchProgress>>,
    then_callback: Option<ThenCallback>,
    catch_callback: Option<CatchCallback>,
}

impl Batch {
    /// Create a new batch with a name
    pub fn new(name: impl Into<String>) -> Self {
        let batch_id = Uuid::new_v4().to_string();
        let name = name.into();

        let progress = BatchProgress {
            batch_id: batch_id.clone(),
            name: name.clone(),
            total_jobs: 0,
            pending_jobs: 0,
            failed_jobs: 0,
            status: BatchStatus::Pending,
            created_at: Utc::now(),
            finished_at: None,
        };

        Self {
            id: batch_id,
            name,
            jobs: Vec::new(),
            progress: Arc::new(RwLock::new(progress)),
            then_callback: None,
            catch_callback: None,
        }
    }

    /// Add jobs to the batch
    pub fn jobs(mut self, jobs: Vec<Arc<dyn Job>>) -> Self {
        let job_count = jobs.len();
        self.jobs = jobs;

        // Update progress
        let progress = Arc::clone(&self.progress);
        tokio::spawn(async move {
            let mut p = progress.write().await;
            p.total_jobs = job_count;
            p.pending_jobs = job_count;
        });

        self
    }

    /// Add a single job
    pub fn add_job(mut self, job: Arc<dyn Job>) -> Self {
        self.jobs.push(job);

        let progress = Arc::clone(&self.progress);
        tokio::spawn(async move {
            let mut p = progress.write().await;
            p.total_jobs += 1;
            p.pending_jobs += 1;
        });

        self
    }

    /// Set callback to execute when all jobs complete successfully
    pub fn then<F>(mut self, callback: F) -> Self
    where
        F: Fn(&BatchProgress) + Send + Sync + 'static,
    {
        self.then_callback = Some(Arc::new(callback));
        self
    }

    /// Set callback to execute when a job fails
    pub fn catch<F>(mut self, callback: F) -> Self
    where
        F: Fn(&BatchProgress, &str) + Send + Sync + 'static,
    {
        self.catch_callback = Some(Arc::new(callback));
        self
    }

    /// Dispatch the batch for execution
    pub async fn dispatch(self) -> Result<BatchHandle> {
        let progress = Arc::clone(&self.progress);

        // Update status to processing
        {
            let mut p = progress.write().await;
            p.status = BatchStatus::Processing;
        }

        // Execute jobs
        let jobs = self.jobs;
        let then_callback = self.then_callback;
        let catch_callback = self.catch_callback;
        let progress_clone = Arc::clone(&progress);

        tokio::spawn(async move {
            let mut has_failures = false;

            for job in jobs {
                let job_name = job.name();

                match job.handle().await {
                    Ok(_) => {
                        // Decrement pending jobs
                        let mut p = progress_clone.write().await;
                        p.pending_jobs = p.pending_jobs.saturating_sub(1);
                    }
                    Err(e) => {
                        has_failures = true;
                        let mut p = progress_clone.write().await;
                        p.pending_jobs = p.pending_jobs.saturating_sub(1);
                        p.failed_jobs += 1;

                        // Call catch callback if set
                        if let Some(ref callback) = catch_callback {
                            callback(&p, &format!("{}: {}", job_name, e));
                        }
                    }
                }
            }

            // Finalize batch
            let mut p = progress_clone.write().await;
            p.finished_at = Some(Utc::now());

            if has_failures {
                p.status = BatchStatus::Failed;
            } else {
                p.status = BatchStatus::Completed;

                // Call then callback if all succeeded
                if let Some(ref callback) = then_callback {
                    callback(&p);
                }
            }
        });

        Ok(BatchHandle {
            id: self.id,
            progress,
        })
    }

    /// Get batch ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get batch name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Handle to a dispatched batch for querying status
#[derive(Clone)]
pub struct BatchHandle {
    id: String,
    progress: Arc<RwLock<BatchProgress>>,
}

impl BatchHandle {
    /// Get the batch ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get current progress (0.0 to 1.0)
    pub async fn progress(&self) -> f64 {
        self.progress.read().await.progress()
    }

    /// Get full progress information
    pub async fn status(&self) -> BatchProgress {
        self.progress.read().await.clone()
    }

    /// Check if batch is finished
    pub async fn is_finished(&self) -> bool {
        self.progress.read().await.is_finished()
    }

    /// Wait for batch to complete
    pub async fn wait(&self) -> BatchProgress {
        loop {
            let progress = self.progress.read().await.clone();
            if progress.is_finished() {
                return progress;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestJob {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<()> {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            if self.should_fail {
                anyhow::bail!("Job failed intentionally");
            }
            Ok(())
        }

        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_batch_creation() {
        let batch = Batch::new("test-batch");
        assert_eq!(batch.name(), "test-batch");
        assert!(!batch.id().is_empty());
    }

    #[tokio::test]
    async fn test_batch_with_successful_jobs() {
        let jobs: Vec<Arc<dyn Job>> = vec![
            Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
            }),
            Arc::new(TestJob {
                name: "job2".to_string(),
                should_fail: false,
            }),
        ];

        let handle = Batch::new("success-batch")
            .jobs(jobs)
            .dispatch()
            .await
            .unwrap();

        let result = handle.wait().await;
        assert_eq!(result.status, BatchStatus::Completed);
        assert_eq!(result.failed_jobs, 0);
        assert_eq!(result.progress(), 1.0);
    }

    #[tokio::test]
    async fn test_batch_with_failed_jobs() {
        let jobs: Vec<Arc<dyn Job>> = vec![
            Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
            }),
            Arc::new(TestJob {
                name: "job2".to_string(),
                should_fail: true,
            }),
        ];

        let handle = Batch::new("failed-batch")
            .jobs(jobs)
            .dispatch()
            .await
            .unwrap();

        let result = handle.wait().await;
        assert_eq!(result.status, BatchStatus::Failed);
        assert_eq!(result.failed_jobs, 1);
    }

    #[tokio::test]
    async fn test_batch_progress_tracking() {
        let jobs: Vec<Arc<dyn Job>> = vec![
            Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
            }),
            Arc::new(TestJob {
                name: "job2".to_string(),
                should_fail: false,
            }),
            Arc::new(TestJob {
                name: "job3".to_string(),
                should_fail: false,
            }),
        ];

        let handle = Batch::new("progress-batch")
            .jobs(jobs)
            .dispatch()
            .await
            .unwrap();

        // Wait a bit and check progress
        tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
        let progress = handle.progress().await;
        assert!(progress > 0.0 && progress <= 1.0);

        let result = handle.wait().await;
        assert_eq!(result.progress(), 1.0);
    }

    #[tokio::test]
    async fn test_batch_then_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let jobs: Vec<Arc<dyn Job>> = vec![
            Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
            }),
        ];

        let handle = Batch::new("then-batch")
            .jobs(jobs)
            .then(move |_| {
                called_clone.store(true, Ordering::SeqCst);
            })
            .dispatch()
            .await
            .unwrap();

        handle.wait().await;
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_batch_catch_callback() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let called = Arc::new(AtomicBool::new(false));
        let called_clone = Arc::clone(&called);

        let jobs: Vec<Arc<dyn Job>> = vec![
            Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: true,
            }),
        ];

        let handle = Batch::new("catch-batch")
            .jobs(jobs)
            .catch(move |_, _| {
                called_clone.store(true, Ordering::SeqCst);
            })
            .dispatch()
            .await
            .unwrap();

        handle.wait().await;
        assert!(called.load(Ordering::SeqCst));
    }
}
