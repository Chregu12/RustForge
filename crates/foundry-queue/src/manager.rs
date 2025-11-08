use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::backends::{QueueBackend, MemoryBackend, RedisBackend};
use crate::error::{QueueError, QueueResult};
use crate::job::Job;

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Queue driver (memory, redis, database)
    pub driver: String,
    /// Redis connection URL (for Redis driver)
    pub redis_url: Option<String>,
    /// Queue prefix
    pub prefix: String,
    /// Default queue name
    pub default_queue: String,
    /// Default timeout in seconds
    pub timeout: u64,
}

impl QueueConfig {
    /// Create from environment variables
    pub fn from_env() -> QueueResult<Self> {
        let driver = std::env::var("QUEUE_DRIVER").unwrap_or_else(|_| "memory".to_string());
        let redis_url = std::env::var("REDIS_URL").ok();
        let prefix = std::env::var("QUEUE_PREFIX").unwrap_or_else(|_| "queue:".to_string());
        let default_queue = std::env::var("QUEUE_DEFAULT").unwrap_or_else(|_| "default".to_string());
        let timeout = std::env::var("QUEUE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        Ok(Self {
            driver,
            redis_url,
            prefix,
            default_queue,
            timeout,
        })
    }

    /// Create default in-memory config
    pub fn memory() -> Self {
        Self {
            driver: "memory".to_string(),
            redis_url: None,
            prefix: "queue:".to_string(),
            default_queue: "default".to_string(),
            timeout: 300,
        }
    }

    /// Create Redis config
    pub fn redis(url: impl Into<String>) -> Self {
        Self {
            driver: "redis".to_string(),
            redis_url: Some(url.into()),
            prefix: "queue:".to_string(),
            default_queue: "default".to_string(),
            timeout: 300,
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self::memory()
    }
}

/// Queue manager for dispatching and managing jobs
pub struct QueueManager {
    backend: Arc<dyn QueueBackend>,
    config: QueueConfig,
}

impl QueueManager {
    /// Create a new queue manager with the given backend
    pub fn new(backend: Arc<dyn QueueBackend>, config: QueueConfig) -> Self {
        Self { backend, config }
    }

    /// Create from configuration
    pub fn from_config(config: QueueConfig) -> QueueResult<Self> {
        let backend: Arc<dyn QueueBackend> = match config.driver.as_str() {
            "redis" => {
                let url = config
                    .redis_url
                    .as_ref()
                    .ok_or_else(|| QueueError::Config("Redis URL required for redis driver".to_string()))?;
                Arc::new(RedisBackend::with_prefix(url, &config.prefix)?)
            }
            "memory" => Arc::new(MemoryBackend::new()),
            _ => {
                return Err(QueueError::Config(format!(
                    "Unknown queue driver: {}",
                    config.driver
                )));
            }
        };

        Ok(Self::new(backend, config))
    }

    /// Create from environment variables
    pub fn from_env() -> QueueResult<Self> {
        let config = QueueConfig::from_env()?;
        Self::from_config(config)
    }

    /// Dispatch a job to the queue
    pub async fn dispatch(&self, job: Job) -> QueueResult<String> {
        let job_id = job.id.clone();
        self.backend.push(job).await?;
        Ok(job_id)
    }

    /// Dispatch multiple jobs
    pub async fn dispatch_many(&self, jobs: Vec<Job>) -> QueueResult<Vec<String>> {
        let mut ids = Vec::new();
        for job in jobs {
            let id = self.dispatch(job).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    /// Get a job by ID
    pub async fn get_job(&self, job_id: &str) -> QueueResult<Option<Job>> {
        self.backend.get(job_id).await
    }

    /// Delete a job
    pub async fn delete_job(&self, job_id: &str) -> QueueResult<bool> {
        self.backend.delete(job_id).await
    }

    /// Get queue size
    pub async fn size(&self) -> QueueResult<usize> {
        self.backend.size().await
    }

    /// Get size of specific queue
    pub async fn size_of(&self, queue: &str) -> QueueResult<usize> {
        self.backend.size_of(queue).await
    }

    /// Clear all queues
    pub async fn clear(&self) -> QueueResult<()> {
        self.backend.clear().await
    }

    /// Clear specific queue
    pub async fn clear_queue(&self, queue: &str) -> QueueResult<()> {
        self.backend.clear_queue(queue).await
    }

    /// Get all failed jobs
    pub async fn failed_jobs(&self) -> QueueResult<Vec<Job>> {
        self.backend.get_failed().await
    }

    /// Get the backend (for advanced usage)
    pub fn backend(&self) -> &Arc<dyn QueueBackend> {
        &self.backend
    }

    /// Get configuration
    pub fn config(&self) -> &QueueConfig {
        &self.config
    }
}

impl Clone for QueueManager {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_queue_manager_memory() {
        let config = QueueConfig::memory();
        let manager = QueueManager::from_config(config).unwrap();

        let job = Job::new("test").with_payload(json!({"key": "value"}));
        let job_id = manager.dispatch(job).await.unwrap();

        assert!(!job_id.is_empty());
        assert_eq!(manager.size().await.unwrap(), 1);

        let retrieved = manager.get_job(&job_id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_dispatch_many() {
        let manager = QueueManager::from_config(QueueConfig::memory()).unwrap();

        let jobs = vec![
            Job::new("job1"),
            Job::new("job2"),
            Job::new("job3"),
        ];

        let ids = manager.dispatch_many(jobs).await.unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(manager.size().await.unwrap(), 3);
    }

    #[test]
    fn test_config_from_env() {
        std::env::set_var("QUEUE_DRIVER", "memory");
        std::env::set_var("QUEUE_PREFIX", "test:");

        let config = QueueConfig::from_env().unwrap();
        assert_eq!(config.driver, "memory");
        assert_eq!(config.prefix, "test:");

        std::env::remove_var("QUEUE_DRIVER");
        std::env::remove_var("QUEUE_PREFIX");
    }
}
