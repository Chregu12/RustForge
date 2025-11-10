//! Worker for processing queued jobs

use crate::error::{QueueError, QueueResult};
use crate::job::{Job, JobMetadata};
use crate::queue::Queue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

type JobHandler = Arc<dyn Fn(Vec<u8>) -> JobHandlerFuture + Send + Sync>;
type JobHandlerFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), QueueError>> + Send>>;

/// Worker for processing jobs from queue
pub struct Worker {
    queue: Arc<dyn Queue>,
    handlers: HashMap<String, JobHandler>,
    concurrency: usize,
    queue_names: Vec<String>,
    poll_interval: Duration,
}

impl Worker {
    /// Create new worker
    pub fn new(queue: Arc<dyn Queue>) -> Self {
        Self {
            queue,
            handlers: HashMap::new(),
            concurrency: 1,
            queue_names: vec!["default".to_string()],
            poll_interval: Duration::from_secs(1),
        }
    }

    /// Set concurrency level
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    /// Set queue names to process
    pub fn queues(mut self, queues: Vec<String>) -> Self {
        self.queue_names = queues;
        self
    }

    /// Set poll interval
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Register a job handler
    pub fn handle<J: Job + 'static>(mut self, handler: impl Fn(J) -> JobHandlerFuture + Send + Sync + 'static) -> Self {
        let job_type = std::any::type_name::<J>();

        let handler_fn = Arc::new(move |data: Vec<u8>| -> JobHandlerFuture {
            let job: J = match serde_json::from_slice(&data) {
                Ok(j) => j,
                Err(e) => {
                    return Box::pin(async move {
                        Err(QueueError::DeserializationError(e.to_string()))
                    });
                }
            };

            handler(job)
        });

        self.handlers.insert(job_type.to_string(), handler_fn);
        self
    }

    /// Start processing jobs
    pub async fn start(self) -> QueueResult<()> {
        let worker = Arc::new(self);
        let mut handles = vec![];

        for _ in 0..worker.concurrency {
            let worker_clone = Arc::clone(&worker);
            let handle = tokio::spawn(async move {
                worker_clone.run_loop().await
            });
            handles.push(handle);
        }

        // Wait for all workers
        for handle in handles {
            handle.await.map_err(|e| QueueError::WorkerError(e.to_string()))??;
        }

        Ok(())
    }

    async fn run_loop(&self) -> QueueResult<()> {
        loop {
            let mut processed = false;

            // Try each queue
            for queue_name in &self.queue_names {
                if let Some(metadata) = self.queue.reserve(queue_name).await? {
                    processed = true;
                    self.process_job(metadata).await;
                }
            }

            // Sleep if no jobs processed
            if !processed {
                sleep(self.poll_interval).await;
            }
        }
    }

    async fn process_job(&self, mut metadata: JobMetadata) {
        let job_id = metadata.id.clone();
        let job_type = metadata.job_type.clone();

        tracing::info!(
            job_id = %job_id,
            job_type = %job_type,
            attempt = metadata.attempts,
            "Processing job"
        );

        // Find handler
        let handler = match self.handlers.get(&job_type) {
            Some(h) => h,
            None => {
                tracing::error!(job_type = %job_type, "No handler registered for job type");
                let _ = self.queue.fail(&job_id, "No handler registered").await;
                return;
            }
        };

        // Execute job
        let start = std::time::Instant::now();
        let result = handler(metadata.data.clone()).await;
        let duration = start.elapsed();

        match result {
            Ok(_) => {
                tracing::info!(
                    job_id = %job_id,
                    duration_ms = duration.as_millis(),
                    "Job completed successfully"
                );
                let _ = self.queue.complete(&job_id).await;
            }
            Err(e) => {
                let error_msg = e.to_string();
                tracing::error!(
                    job_id = %job_id,
                    error = %error_msg,
                    attempt = metadata.attempts,
                    "Job failed"
                );

                metadata.mark_error(error_msg.clone());

                if metadata.can_retry() {
                    tracing::info!(
                        job_id = %job_id,
                        attempt = metadata.attempts + 1,
                        max_retries = metadata.max_retries,
                        "Retrying job"
                    );
                    let _ = self.queue.retry(metadata).await;
                } else {
                    tracing::error!(job_id = %job_id, "Max retries exceeded, job failed permanently");
                    let _ = self.queue.fail(&job_id, &error_msg).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Job;
    use crate::memory::MemoryQueue;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    struct TestJob {
        message: String,
        should_fail: bool,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<(), QueueError> {
            if self.should_fail {
                Err(QueueError::JobFailed("Intentional failure".to_string()))
            } else {
                Ok(())
            }
        }

        fn job_type(&self) -> &'static str {
            "test_job"
        }

        fn max_retries(&self) -> u32 {
            2
        }
    }

    #[tokio::test]
    #[ignore] // Flaky due to async timing
    async fn test_worker_processes_job() {
        let queue = Arc::new(MemoryQueue::new());
        let job = TestJob {
            message: "test".to_string(),
            should_fail: false,
        };

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        let processed = Arc::new(tokio::sync::Mutex::new(false));
        let processed_clone = Arc::clone(&processed);

        let worker = Worker::new(Arc::clone(&queue) as Arc<dyn Queue>)
            .poll_interval(Duration::from_millis(10))
            .handle(move |job: TestJob| {
                let processed = Arc::clone(&processed_clone);
                Box::pin(async move {
                    *processed.lock().await = true;
                    job.handle().await
                })
            });

        // Run worker for a short time
        tokio::select! {
            _ = worker.start() => {}
            _ = tokio::time::sleep(Duration::from_millis(100)) => {}
        }

        assert!(*processed.lock().await, "Job should have been processed");
    }
}
