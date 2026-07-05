//! Worker for processing queued jobs

use crate::error::{QueueError, QueueResult};
use crate::job::{Job, JobMetadata};
use crate::queue::Queue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

type JobHandler = Arc<dyn Fn(Vec<u8>) -> JobHandlerFuture + Send + Sync>;
type JobHandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), QueueError>> + Send>>;

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
    pub fn handle<J: Job + 'static>(
        mut self,
        handler: impl Fn(J) -> JobHandlerFuture + Send + Sync + 'static,
    ) -> Self {
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

    /// Register a job type so the worker can execute its [`Job::handle`] directly.
    ///
    /// This is the ergonomic counterpart to [`Job::dispatch`]: you dispatch a
    /// job with `job.dispatch(&queue).await`, register its type on a worker with
    /// `Worker::new(queue).register::<MyJob>()`, and the worker deserializes the
    /// payload back into `MyJob` and runs `handle()` for real, in-process.
    pub fn register<J: Job + 'static>(mut self) -> Self {
        let key = std::any::type_name::<J>().to_string();

        let handler_fn = Arc::new(move |data: Vec<u8>| -> JobHandlerFuture {
            Box::pin(async move {
                let job: J = serde_json::from_slice(&data)
                    .map_err(|e| QueueError::DeserializationError(e.to_string()))?;
                job.handle().await
            })
        });

        self.handlers.insert(key, handler_fn);
        self
    }

    /// Process at most one ready job across the configured queues.
    ///
    /// Reserves the next available job (respecting delayed `execute_at`),
    /// executes its handler, and reports the outcome to the queue backend
    /// (`complete`/`retry`/`fail`). Returns `Ok(true)` if a job was processed,
    /// `Ok(false)` if every configured queue was empty (a safe no-op).
    pub async fn work_once(&self) -> QueueResult<bool> {
        for queue_name in &self.queue_names {
            if let Some(metadata) = self.queue.reserve(queue_name).await? {
                self.process_job(metadata).await;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Start processing jobs
    pub async fn start(self) -> QueueResult<()> {
        let worker = Arc::new(self);
        let mut handles = vec![];

        for _ in 0..worker.concurrency {
            let worker_clone = Arc::clone(&worker);
            let handle = tokio::spawn(async move { worker_clone.run_loop().await });
            handles.push(handle);
        }

        // Wait for all workers
        for handle in handles {
            handle
                .await
                .map_err(|e| QueueError::WorkerError(e.to_string()))??;
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

        // Route to a handler by the concrete Rust type key (handler_key), which is
        // how handlers are registered. Fall back to the user-facing job_type for
        // payloads serialized before handler_key existed.
        let lookup_key = if metadata.handler_key.is_empty() {
            job_type.as_str()
        } else {
            metadata.handler_key.as_str()
        };
        let handler = match self.handlers.get(lookup_key) {
            Some(h) => h,
            None => {
                tracing::error!(job_type = %job_type, "No handler registered for job type");
                let _ = self.queue.fail(&job_id, "No handler registered").await;
                return;
            }
        };

        // Execute job with timeout enforcement
        let start = std::time::Instant::now();
        let timeout_secs = metadata.timeout_secs.max(1);
        let result = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            handler(metadata.data.clone()),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => Err(QueueError::Timeout(timeout_secs)),
        };
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

    #[cfg(test)]
    #[cfg(feature = "redis-backend")]
    async fn redis_available() -> bool {
        use redis::AsyncCommands;
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(mut conn) => conn.ping::<_, String>().await.is_ok(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    use super::*;
    use crate::job::Job;
    use crate::memory::MemoryQueue;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // A process-wide side-effect sink so the test can prove the worker actually
    // executed the job body (not merely dequeued it).
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    static LAST_PAYLOAD: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

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
                COUNTER.fetch_add(1, Ordering::SeqCst);
                *LAST_PAYLOAD.lock().unwrap() = self.message.clone();
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

    // Single sequential test: the side-effect sinks (COUNTER/LAST_PAYLOAD) are
    // process-global because `Job::handle(&self)` has no external state handle,
    // so running the scenarios in one test avoids cross-test races.
    #[tokio::test]
    async fn test_worker_executes_jobs_fifo_and_empty_is_noop() {
        COUNTER.store(0, Ordering::SeqCst);
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let worker = Worker::new(Arc::clone(&queue)).register::<TestJob>();

        // Empty queue -> safe no-op, no side effect.
        assert!(!worker.work_once().await.unwrap(), "empty queue is a no-op");
        assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

        // Dispatch three jobs and prove FIFO execution of handle() with payload.
        for msg in ["first", "second", "third"] {
            TestJob {
                message: msg.to_string(),
                should_fail: false,
            }
            .dispatch(&queue)
            .await
            .unwrap();
        }

        assert!(worker.work_once().await.unwrap());
        assert_eq!(&*LAST_PAYLOAD.lock().unwrap(), "first");
        assert_eq!(COUNTER.load(Ordering::SeqCst), 1);

        assert!(worker.work_once().await.unwrap());
        assert_eq!(&*LAST_PAYLOAD.lock().unwrap(), "second");

        assert!(worker.work_once().await.unwrap());
        assert_eq!(&*LAST_PAYLOAD.lock().unwrap(), "third");

        // Drained -> no-op again; handle() ran exactly three times total.
        assert!(!worker.work_once().await.unwrap());
        assert_eq!(COUNTER.load(Ordering::SeqCst), 3);
    }
}
