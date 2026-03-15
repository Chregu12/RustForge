//! Worker pool for processing jobs

use crate::context::JobContext;
use crate::error::{JobError, WorkerError};
use crate::job::JobPayload;
use crate::queue::QueueManager;
use crate::registry::JobRegistry;
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
    // Retained for potential future use (e.g., dynamic worker scaling, stats)
    #[allow(dead_code)]
    queue_manager: Arc<QueueManager>,
    #[allow(dead_code)]
    registry: Arc<JobRegistry>,
    workers: Vec<Worker>,
    shutdown_tx: broadcast::Sender<()>,
}

impl WorkerPool {
    /// Create new worker pool
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use rf_jobs::{WorkerPool, WorkerConfig, QueueManager, JobRegistry};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = QueueManager::new("redis://localhost:6379").await?;
    /// let registry = JobRegistry::new();
    /// let config = WorkerConfig::default().workers(4);
    /// let pool = WorkerPool::new(config, manager, registry).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        config: WorkerConfig,
        queue_manager: QueueManager,
        registry: JobRegistry,
    ) -> Result<Self, WorkerError> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let queue_manager = Arc::new(queue_manager);
        let registry = Arc::new(registry);

        let mut workers = Vec::new();
        for i in 0..config.workers {
            let worker = Worker::new(
                i,
                config.clone(),
                Arc::clone(&queue_manager),
                Arc::clone(&registry),
                shutdown_tx.subscribe(),
            );
            workers.push(worker);
        }

        Ok(Self {
            config,
            queue_manager,
            registry,
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
    registry: Arc<JobRegistry>,
    shutdown_rx: broadcast::Receiver<()>,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Create new worker
    fn new(
        id: usize,
        config: WorkerConfig,
        queue_manager: Arc<QueueManager>,
        registry: Arc<JobRegistry>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            id,
            config,
            queue_manager,
            registry,
            shutdown_rx,
            handle: None,
        }
    }

    /// Start worker
    async fn start(&mut self) -> Result<(), WorkerError> {
        let id = self.id;
        let config = self.config.clone();
        let queue_manager = Arc::clone(&self.queue_manager);
        let registry = Arc::clone(&self.registry);
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
                            Self::process_job(id, payload, &queue_manager, &registry, &config)
                                .await;
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
        registry: &JobRegistry,
        config: &WorkerConfig,
    ) {
        tracing::info!(
            worker = worker_id,
            job_id = %payload.id,
            job_type = %payload.job_type,
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
            Self::execute_job_payload(&payload, ctx.clone(), registry),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                // Job succeeded
                tracing::info!(
                    worker = worker_id,
                    job_id = %payload.id,
                    job_type = %payload.job_type,
                    "Job completed successfully"
                );
            }
            Ok(Err(job_error)) => {
                // Job failed
                tracing::error!(
                    worker = worker_id,
                    job_id = %payload.id,
                    job_type = %payload.job_type,
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
                    job_type = %payload.job_type,
                    timeout = ?config.timeout,
                    "Job timed out"
                );

                Self::handle_failed_job(payload, error, queue_manager).await;
            }
        }
    }

    /// Execute job payload using the registry
    ///
    /// This is the critical method that actually executes jobs!
    /// It uses the registry to deserialize and dispatch to the correct handler.
    async fn execute_job_payload(
        payload: &JobPayload,
        ctx: JobContext,
        registry: &JobRegistry,
    ) -> Result<(), JobError> {
        // Extract the payload data as JSON string
        let payload_str = payload.data.to_string();

        // Use registry to execute the job
        // This will:
        // 1. Look up the handler for this job type
        // 2. Deserialize the payload to the concrete job type
        // 3. Call the job's handle() method
        registry
            .execute(&payload.job_type, &payload_str, ctx.clone())
            .await?;

        Ok(())
    }

    /// Handle failed job (retry or move to DLQ)
    ///
    /// CRITICAL FIX: This now preserves the original payload when retrying!
    async fn handle_failed_job(
        mut payload: JobPayload,
        error: JobError,
        queue_manager: &QueueManager,
    ) {
        if payload.has_more_attempts() {
            // Retry job with ORIGINAL PAYLOAD preserved
            tracing::warn!(
                job_id = %payload.id,
                job_type = %payload.job_type,
                attempt = payload.attempt,
                max_attempts = payload.max_attempts,
                "Retrying job"
            );

            // Calculate exponential backoff
            let backoff_multiplier = 2u64.pow(payload.attempt);
            let delay_seconds = payload.backoff_seconds * backoff_multiplier;

            // Update available_at for delayed retry
            let delay = chrono::Duration::seconds(delay_seconds as i64);
            payload.available_at = chrono::Utc::now() + delay;

            // Re-queue the SAME payload (not a DummyJob!)
            // This preserves all job data and metadata
            if let Err(e) = queue_manager
                .push_raw(&payload.queue, payload.clone())
                .await
            {
                tracing::error!(
                    job_id = %payload.id,
                    job_type = %payload.job_type,
                    error = %e,
                    "Failed to requeue job for retry"
                );
            } else {
                tracing::info!(
                    job_id = %payload.id,
                    job_type = %payload.job_type,
                    retry_in_seconds = delay_seconds,
                    "Job requeued for retry"
                );
            }
        } else {
            // Move to failed queue
            tracing::error!(
                job_id = %payload.id,
                job_type = %payload.job_type,
                "Job failed permanently after {} attempts, moving to failed queue",
                payload.attempt
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
