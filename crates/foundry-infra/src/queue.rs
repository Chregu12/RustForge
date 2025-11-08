use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use foundry_plugins::{CommandError, QueueJob, QueuePort};
use foundry_queue::{Job, QueueManager};
use tracing::info;

#[derive(Clone, Default)]
pub struct InMemoryQueue {
    jobs: Arc<Mutex<Vec<QueueJob>>>,
}

impl InMemoryQueue {
    pub fn jobs(&self) -> Vec<QueueJob> {
        self.jobs.lock().unwrap().clone()
    }
}

#[async_trait]
impl QueuePort for InMemoryQueue {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError> {
        info!(name = %job.name, "Queue job dispatched");
        self.jobs.lock().unwrap().push(job);
        Ok(())
    }
}

/// Redis Queue adapter that implements QueuePort
#[derive(Clone)]
pub struct RedisQueue {
    manager: QueueManager,
}

impl RedisQueue {
    /// Create a new Redis queue from configuration
    pub fn new(manager: QueueManager) -> Self {
        Self { manager }
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self, CommandError> {
        let manager = QueueManager::from_env()
            .map_err(|e| CommandError::Message(format!("Failed to create queue manager: {}", e)))?;
        Ok(Self::new(manager))
    }

    /// Get the underlying queue manager
    pub fn manager(&self) -> &QueueManager {
        &self.manager
    }
}

#[async_trait]
impl QueuePort for RedisQueue {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError> {
        info!(name = %job.name, "Queue job dispatched to Redis");

        // Convert QueueJob to foundry_queue::Job
        let queue_job = Job::new(&job.name)
            .with_payload(job.payload);

        // Apply delay if specified
        let queue_job = if let Some(delay_seconds) = job.delay_seconds {
            queue_job.with_delay(std::time::Duration::from_secs(delay_seconds))
        } else {
            queue_job
        };

        // Dispatch to queue
        self.manager
            .dispatch(queue_job)
            .await
            .map_err(|e| CommandError::Message(format!("Queue dispatch failed: {}", e)))?;

        Ok(())
    }
}
