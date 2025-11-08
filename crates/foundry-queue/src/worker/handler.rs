use async_trait::async_trait;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{QueueError, QueueResult};
use crate::job::Job;

/// Job handler trait
#[async_trait]
pub trait JobHandler: Send + Sync {
    /// Handle a job
    async fn handle(&self, job: &Job) -> QueueResult<Option<Value>>;
}

/// Registry for job handlers
pub struct JobHandlerRegistry {
    handlers: Arc<RwLock<FxHashMap<String, Arc<dyn JobHandler>>>>,
}

impl JobHandlerRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Register a handler for a job type
    pub fn register<H: JobHandler + 'static>(&self, name: impl Into<String>, handler: H) {
        let mut handlers = self.handlers.blocking_write();
        handlers.insert(name.into(), Arc::new(handler));
    }

    /// Handle a job by routing to the appropriate handler
    pub async fn handle(&self, job: &Job) -> QueueResult<Option<Value>> {
        let handlers = self.handlers.read().await;

        if let Some(handler) = handlers.get(&job.name) {
            handler.handle(job).await
        } else {
            Err(QueueError::Worker(format!(
                "No handler registered for job type: {}",
                job.name
            )))
        }
    }

    /// Check if a handler is registered
    pub async fn has_handler(&self, name: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(name)
    }
}

impl Default for JobHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Example handler implementations
pub mod handlers {
    use super::*;
    use tracing::info;

    /// Echo handler - logs the job and returns the payload
    pub struct EchoHandler;

    #[async_trait]
    impl JobHandler for EchoHandler {
        async fn handle(&self, job: &Job) -> QueueResult<Option<Value>> {
            info!(
                job_id = %job.id,
                job_name = %job.name,
                payload = ?job.payload,
                "Echo handler processing job"
            );
            Ok(Some(job.payload.clone()))
        }
    }

    /// Noop handler - does nothing
    pub struct NoopHandler;

    #[async_trait]
    impl JobHandler for NoopHandler {
        async fn handle(&self, _job: &Job) -> QueueResult<Option<Value>> {
            Ok(None)
        }
    }

    /// Failing handler - always fails (for testing)
    pub struct FailingHandler;

    #[async_trait]
    impl JobHandler for FailingHandler {
        async fn handle(&self, _job: &Job) -> QueueResult<Option<Value>> {
            Err(QueueError::Worker("Intentional failure".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Job;

    struct TestHandler {
        result: String,
    }

    #[async_trait]
    impl JobHandler for TestHandler {
        async fn handle(&self, _job: &Job) -> QueueResult<Option<Value>> {
            Ok(Some(serde_json::json!({"result": self.result})))
        }
    }

    #[tokio::test]
    async fn test_handler_registry() {
        let registry = JobHandlerRegistry::new();

        registry.register(
            "test",
            TestHandler {
                result: "success".to_string(),
            },
        );

        assert!(registry.has_handler("test").await);
        assert!(!registry.has_handler("other").await);
    }

    #[tokio::test]
    async fn test_handler_execution() {
        let registry = JobHandlerRegistry::new();

        registry.register(
            "test",
            TestHandler {
                result: "success".to_string(),
            },
        );

        let job = Job::new("test");
        let result = registry.handle(&job).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap()["result"], "success");
    }

    #[tokio::test]
    async fn test_unknown_handler() {
        let registry = JobHandlerRegistry::new();
        let job = Job::new("unknown");

        let result = registry.handle(&job).await;
        assert!(result.is_err());
    }
}
