//! Worker pool for processing jobs

use crate::context::JobContext;
use crate::error::{JobError, WorkerError};
use crate::job::{Job, JobPayload};
use crate::queue::QueueManager;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of concurrent workers
    pub workers: usize,

    /// Queues to listen on (in priority order)
    pub queues: Vec<String>,

    /// Max job execution time
    pub timeout: Duration,

    /// Sleep time when no jobs available
    pub sleep: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            workers: num_cpus::get(),
            queues: vec!["default".to_string()],
            timeout: Duration::from_secs(60),
            sleep: Duration::from_secs(1),
        }
    }
}

impl WorkerConfig {
    /// Set number of workers
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }

    /// Set queues to listen on
    pub fn queues(mut self, queues: &[&str]) -> Self {
        self.queues = queues.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set job timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set sleep duration
    pub fn sleep(mut self, sleep: Duration) -> Self {
        self.sleep = sleep;
        self
    }
}

/// Worker pool for job processing
pub struct WorkerPool {
    config: WorkerConfig,
    queue_manager: Arc<QueueManager>,
    workers: Vec<Worker>,
    shutdown_tx: broadcast::Sender<()>,
}

impl WorkerPool {
    /// Create new worker pool
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use rf_jobs::{WorkerPool, WorkerConfig, QueueManager};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = QueueManager::new("redis://localhost:6379").await?;
    /// let config = WorkerConfig::default().workers(4);
    /// let pool = WorkerPool::new(config, manager).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        config: WorkerConfig,
        queue_manager: QueueManager,
    ) -> Result<Self, WorkerError> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let queue_manager = Arc::new(queue_manager);

        let mut workers = Vec::new();
        for i in 0..config.workers {
            let worker = Worker::new(
                i,
                config.clone(),
                Arc::clone(&queue_manager),
                shutdown_tx.subscribe(),
            );
            workers.push(worker);
        }

        Ok(Self {
            config,
            queue_manager,
            workers,
            shutdown_tx,
        })
    }

    /// Start all workers
    pub async fn start(&mut self) -> Result<(), WorkerError> {
        tracing::info!(
            "Starting {} workers for queues: {:?}",
            self.config.workers,
            self.config.queues
        );

        for worker in &mut self.workers {
            worker.start().await?;
        }

        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(self) -> Result<(), WorkerError> {
        tracing::info!("Shutting down worker pool");

        // Signal all workers to stop
        let _ = self.shutdown_tx.send(());

        // Wait for workers to finish current jobs
        for worker in self.workers {
            worker.wait().await?;
        }

        tracing::info!("Worker pool shutdown complete");
        Ok(())
    }
}

/// Individual worker
pub struct Worker {
    id: usize,
    config: WorkerConfig,
    queue_manager: Arc<QueueManager>,
    shutdown_rx: broadcast::Receiver<()>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Create new worker
    fn new(
        id: usize,
        config: WorkerConfig,
        queue_manager: Arc<QueueManager>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            id,
            config,
            queue_manager,
            shutdown_rx,
            handle: None,
        }
    }

    /// Start worker
    async fn start(&mut self) -> Result<(), WorkerError> {
        let id = self.id;
        let config = self.config.clone();
        let queue_manager = Arc::clone(&self.queue_manager);
        let mut shutdown_rx = self.shutdown_rx.resubscribe();

        let handle = tokio::spawn(async move {
            tracing::info!(worker = id, "Worker started");

            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    tracing::info!(worker = id, "Worker received shutdown signal");
                    break;
                }

                // Try to process job from each queue (priority order)
                let mut processed = false;

                for queue in &config.queues {
                    match queue_manager.pop_nowait(queue).await {
                        Ok(Some(payload)) => {
                            Self::process_job(id, payload, &queue_manager, &config).await;
                            processed = true;
                            break; // Process one job at a time
                        }
                        Ok(None) => {
                            // No job in this queue, try next
                            continue;
                        }
                        Err(e) => {
                            tracing::error!(
                                worker = id,
                                queue = %queue,
                                error = %e,
                                "Failed to pop job from queue"
                            );
                        }
                    }
                }

                // If no jobs processed, sleep
                if !processed {
                    tokio::time::sleep(config.sleep).await;
                }

                // Check delayed jobs periodically
                if let Err(e) = queue_manager.move_delayed_jobs().await {
                    tracing::error!(
                        worker = id,
                        error = %e,
                        "Failed to move delayed jobs"
                    );
                }
            }

            tracing::info!(worker = id, "Worker stopped");
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Process a single job
    async fn process_job(
        worker_id: usize,
        mut payload: JobPayload,
        queue_manager: &QueueManager,
        config: &WorkerConfig,
    ) {
        tracing::info!(
            worker = worker_id,
            job_id = %payload.id,
            queue = %payload.queue,
            attempt = payload.attempt + 1,
            "Processing job"
        );

        // Increment attempt counter
        payload.increment_attempt();

        // Create job context
        let ctx = JobContext::new(
            payload.id,
            payload.queue.clone(),
            payload.attempt,
            payload.max_attempts,
            payload.dispatched_at,
        );

        // Execute job with timeout
        let result = tokio::time::timeout(
            config.timeout,
            Self::execute_job_payload(&payload, ctx.clone()),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                // Job succeeded
                tracing::info!(
                    worker = worker_id,
                    job_id = %payload.id,
                    "Job completed successfully"
                );
            }
            Ok(Err(job_error)) => {
                // Job failed
                tracing::error!(
                    worker = worker_id,
                    job_id = %payload.id,
                    error = %job_error,
                    attempt = payload.attempt,
                    max_attempts = payload.max_attempts,
                    "Job failed"
                );

                Self::handle_failed_job(payload, job_error, queue_manager).await;
            }
            Err(_) => {
                // Job timeout
                let error = JobError::Timeout(config.timeout);

                tracing::error!(
                    worker = worker_id,
                    job_id = %payload.id,
                    timeout = ?config.timeout,
                    "Job timed out"
                );

                Self::handle_failed_job(payload, error, queue_manager).await;
            }
        }
    }

    /// Execute job payload (type-erased)
    async fn execute_job_payload(
        payload: &JobPayload,
        ctx: JobContext,
    ) -> Result<(), JobError> {
        // This is a simplified version - in reality, we would need
        // a job registry to deserialize and execute jobs dynamically

        // For now, we just log that we would execute the job
        ctx.log(&format!(
            "Would execute job of type: {}",
            payload.job_type
        ));

        // Simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Handle failed job (retry or move to DLQ)
    async fn handle_failed_job(
        payload: JobPayload,
        error: JobError,
        queue_manager: &QueueManager,
    ) {
        if payload.has_more_attempts() {
            // Retry job
            tracing::warn!(
                job_id = %payload.id,
                attempt = payload.attempt,
                max_attempts = payload.max_attempts,
                "Retrying job"
            );

            // Push back to queue with backoff
            if let Err(e) = queue_manager
                .dispatch_later(
                    DummyJob,
                    Duration::from_secs(payload.backoff_seconds),
                )
                .await
            {
                tracing::error!(
                    job_id = %payload.id,
                    error = %e,
                    "Failed to requeue job"
                );
            }
        } else {
            // Move to failed queue
            tracing::error!(
                job_id = %payload.id,
                "Job failed permanently, moving to failed queue"
            );

            if let Err(e) = queue_manager
                .add_failed_job(payload, error.to_string())
                .await
            {
                tracing::error!(
                    error = %e,
                    "Failed to add job to failed queue"
                );
            }
        }
    }

    /// Wait for worker to finish
    async fn wait(self) -> Result<(), WorkerError> {
        if let Some(handle) = self.handle {
            handle
                .await
                .map_err(|e| WorkerError::ShutdownError(e.to_string()))?;
        }
        Ok(())
    }
}

// Dummy job for requeuing (temporary workaround)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DummyJob;

#[async_trait::async_trait]
impl Job for DummyJob {
    async fn handle(&self, _ctx: JobContext) -> crate::JobResult {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_config_builder() {
        let config = WorkerConfig::default()
            .workers(4)
            .queues(&["default", "emails"])
            .timeout(Duration::from_secs(30));

        assert_eq!(config.workers, 4);
        assert_eq!(config.queues, vec!["default", "emails"]);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_default_config() {
        let config = WorkerConfig::default();
        assert_eq!(config.queues, vec!["default"]);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }
}
