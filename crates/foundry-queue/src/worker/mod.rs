use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

use crate::backends::QueueBackend;
use crate::error::QueueResult;
use crate::job::{Job, JobResult};
use crate::manager::QueueManager;

pub mod handler;
pub use handler::{JobHandler, JobHandlerRegistry};

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Queues to process (in priority order)
    pub queues: Vec<String>,
    /// Sleep duration when no jobs are available (in seconds)
    pub sleep_duration: Duration,
    /// Maximum number of retries for failed jobs
    pub max_retries: u32,
    /// Worker timeout (None = run indefinitely)
    pub timeout: Option<Duration>,
    /// Process delayed jobs
    pub process_delayed: bool,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            queues: vec!["default".to_string()],
            sleep_duration: Duration::from_secs(1),
            max_retries: 3,
            timeout: None,
            process_delayed: true,
        }
    }
}

/// Queue worker for processing jobs
pub struct Worker {
    backend: Arc<dyn QueueBackend>,
    handler_registry: Arc<JobHandlerRegistry>,
    config: WorkerConfig,
}

impl Worker {
    /// Create a new worker from queue manager
    pub fn new(manager: QueueManager) -> Self {
        Self {
            backend: Arc::clone(manager.backend()),
            handler_registry: Arc::new(JobHandlerRegistry::new()),
            config: WorkerConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(manager: QueueManager, config: WorkerConfig) -> Self {
        Self {
            backend: Arc::clone(manager.backend()),
            handler_registry: Arc::new(JobHandlerRegistry::new()),
            config,
        }
    }

    /// Register a job handler
    pub fn register_handler<H: JobHandler + 'static>(&mut self, name: impl Into<String>, handler: H) {
        Arc::get_mut(&mut self.handler_registry)
            .unwrap()
            .register(name, handler);
    }

    /// Run the worker
    pub async fn run(&self) -> QueueResult<WorkerStats> {
        info!("Starting queue worker");
        let start_time = std::time::Instant::now();
        let mut stats = WorkerStats::default();

        loop {
            // Check timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() >= timeout {
                    info!("Worker timeout reached, shutting down");
                    break;
                }
            }

            // Process delayed jobs
            if self.config.process_delayed {
                if let Err(e) = self.process_delayed_jobs().await {
                    error!("Error processing delayed jobs: {}", e);
                }
            }

            // Try to get a job from queues (in priority order)
            let job = self.pop_job().await?;

            if let Some(mut job) = job {
                // Process the job
                match self.process_job(&mut job).await {
                    Ok(result) => {
                        if result.success {
                            stats.processed += 1;
                            info!(
                                job_id = %job.id,
                                job_name = %job.name,
                                execution_time_ms = result.execution_time_ms,
                                "Job completed successfully"
                            );
                        } else {
                            stats.failed += 1;
                            warn!(
                                job_id = %job.id,
                                job_name = %job.name,
                                error = ?result.error,
                                "Job failed"
                            );
                        }
                    }
                    Err(e) => {
                        stats.failed += 1;
                        error!(
                            job_id = %job.id,
                            job_name = %job.name,
                            error = %e,
                            "Error processing job"
                        );
                    }
                }
            } else {
                // No jobs available, sleep
                sleep(self.config.sleep_duration).await;
            }

            // Check for shutdown signal
            if tokio::signal::ctrl_c().await.is_ok() {
                info!("Received shutdown signal");
                break;
            }
        }

        stats.duration = start_time.elapsed();
        info!(
            processed = stats.processed,
            failed = stats.failed,
            duration_secs = stats.duration.as_secs(),
            "Worker stopped"
        );

        Ok(stats)
    }

    /// Pop a job from the queues
    async fn pop_job(&self) -> QueueResult<Option<Job>> {
        for queue in &self.config.queues {
            if let Some(job) = self.backend.pop_from(queue).await? {
                return Ok(Some(job));
            }
        }
        Ok(None)
    }

    /// Process delayed jobs that are ready
    async fn process_delayed_jobs(&self) -> QueueResult<()> {
        let ready_jobs = self.backend.get_ready_delayed().await?;

        for job in ready_jobs {
            self.backend.release_delayed(&job).await?;
            info!(job_id = %job.id, "Released delayed job");
        }

        Ok(())
    }

    /// Process a single job
    async fn process_job(&self, job: &mut Job) -> QueueResult<JobResult> {
        let start = std::time::Instant::now();

        // Mark job as processing
        job.mark_processing();
        self.backend.update(job).await?;

        // Execute the job
        let result = self.handler_registry.handle(job).await;

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(data) => {
                // Job succeeded
                job.mark_completed();
                self.backend.update(job).await?;

                Ok(JobResult::success(job, data, execution_time_ms))
            }
            Err(err) => {
                // Job failed
                if job.can_retry() {
                    // Can retry, push back to queue
                    info!(
                        job_id = %job.id,
                        attempts = job.attempts,
                        max_attempts = job.max_attempts,
                        "Job failed, will retry"
                    );
                    self.backend.push(job.clone()).await?;
                } else {
                    // Max retries reached
                    job.mark_failed();
                    self.backend.update(job).await?;
                }

                Ok(JobResult::failure(job, err.to_string(), execution_time_ms))
            }
        }
    }
}

/// Worker statistics
#[derive(Debug, Clone, Default)]
pub struct WorkerStats {
    pub processed: u64,
    pub failed: u64,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::MemoryBackend;
    use crate::manager::QueueConfig;
    use serde_json::json;

    #[tokio::test]
    async fn test_worker_creation() {
        let config = QueueConfig::memory();
        let manager = QueueManager::from_config(config).unwrap();
        let _worker = Worker::new(manager);
    }

    #[tokio::test]
    async fn test_worker_with_timeout() {
        let config = QueueConfig::memory();
        let manager = QueueManager::from_config(config).unwrap();

        let worker_config = WorkerConfig {
            timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        };

        let worker = Worker::with_config(manager, worker_config);
        let stats = worker.run().await.unwrap();

        assert!(stats.duration.as_secs() >= 1);
    }
}
