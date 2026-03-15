use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rf_plugins::{CommandError, QueueJob, QueuePort};
use rf_jobs::{JobPayload, QueueManager};
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

    /// Create from environment variables (reads REDIS_URL)
    pub async fn from_env() -> Result<Self, CommandError> {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let manager = QueueManager::new(&redis_url)
            .await
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

        let available_at = if let Some(delay_seconds) = job.delay_seconds {
            chrono::Utc::now() + chrono::Duration::seconds(delay_seconds as i64)
        } else {
            chrono::Utc::now()
        };

        let payload = JobPayload {
            id: uuid::Uuid::new_v4(),
            queue: "default".to_string(),
            job_type: job.name.clone(),
            data: job.payload,
            attempt: 0,
            max_attempts: 3,
            dispatched_at: chrono::Utc::now(),
            available_at,
            backoff_seconds: 60,
        };

        self.manager
            .push_raw("default", payload)
            .await
            .map_err(|e| CommandError::Message(format!("Queue dispatch failed: {}", e)))?;

        Ok(())
    }
}
