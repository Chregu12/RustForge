//! Deployment tests for rf-queue

#[cfg(test)]
mod tests {
    use rf_queue::{Queue, MemoryQueue, Job, JobMetadata, QueueFacade, QueueConfig};
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::Duration;

    #[derive(Serialize, Deserialize, Clone)]
    struct SendEmailJob {
        to: String,
        subject: String,
    }

    #[async_trait]
    impl Job for SendEmailJob {
        async fn handle(&self) -> Result<(), rf_queue::QueueError> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "send_email"
        }

        fn max_retries(&self) -> u32 {
            3
        }

        fn timeout(&self) -> Duration {
            Duration::from_secs(30)
        }

        fn queue(&self) -> &str {
            "emails"
        }

        fn priority(&self) -> i32 {
            1
        }
    }

    // ── MemoryQueue ──────────────────────────────────────────────

    #[tokio::test]
    async fn memory_queue_push_and_reserve() {
        let queue = MemoryQueue::new();
        let job = SendEmailJob {
            to: "test@test.com".into(),
            subject: "Hello".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        let id = queue.push(metadata).await.expect("push");
        assert!(!id.is_empty());

        let size = queue.size("emails").await.expect("size");
        assert_eq!(size, 1);

        let reserved = queue.reserve("emails").await.expect("reserve");
        assert!(reserved.is_some());
    }

    #[tokio::test]
    async fn memory_queue_complete() {
        let queue = MemoryQueue::new();
        let job = SendEmailJob {
            to: "a@b.com".into(),
            subject: "Test".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        let id = queue.push(metadata).await.expect("push");
        queue.complete(&id).await.expect("complete");
    }

    #[tokio::test]
    async fn memory_queue_fail_and_retry() {
        let queue = MemoryQueue::new();
        let job = SendEmailJob {
            to: "a@b.com".into(),
            subject: "Test".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        let id = queue.push(metadata.clone()).await.expect("push");
        queue.fail(&id, "some error").await.expect("fail");
        queue.retry(metadata).await.expect("retry");
    }

    #[tokio::test]
    async fn memory_queue_clear() {
        let queue = MemoryQueue::new();
        let job = SendEmailJob {
            to: "a@b.com".into(),
            subject: "Test".into(),
        };

        queue.push(JobMetadata::new(&job).expect("m")).await.expect("push");
        queue.push(JobMetadata::new(&job).expect("m")).await.expect("push");
        queue.clear("emails").await.expect("clear");
        let size = queue.size("emails").await.expect("size");
        assert_eq!(size, 0);
    }

    // ── JobMetadata ──────────────────────────────────────────────

    #[test]
    fn job_metadata_creation() {
        let job = SendEmailJob {
            to: "test@test.com".into(),
            subject: "Hello".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        assert_eq!(metadata.job_type, "send_email");
        assert_eq!(metadata.queue, "emails");
        assert_eq!(metadata.max_retries, 3);
        assert_eq!(metadata.attempts, 0);
        assert!(metadata.can_retry());
    }

    #[test]
    fn job_metadata_delayed() {
        let job = SendEmailJob {
            to: "test@test.com".into(),
            subject: "Delayed".into(),
        };

        let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(60)).expect("metadata");
        assert!(metadata.execute_at.is_some());
        assert!(!metadata.should_execute()); // should not execute yet (in 60 seconds)
    }

    #[test]
    fn job_metadata_serialization() {
        let job = SendEmailJob {
            to: "test@test.com".into(),
            subject: "Ser".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        let bytes = metadata.to_bytes().expect("serialize");
        let restored = JobMetadata::from_bytes(&bytes).expect("deserialize");
        assert_eq!(restored.job_type, "send_email");
    }

    #[test]
    fn job_metadata_attempts() {
        let job = SendEmailJob {
            to: "t@t.com".into(),
            subject: "T".into(),
        };

        let mut metadata = JobMetadata::new(&job).expect("metadata");
        assert_eq!(metadata.attempts, 0);
        metadata.mark_attempt();
        assert_eq!(metadata.attempts, 1);
        metadata.mark_error("failed".into());
        assert_eq!(metadata.last_error, Some("failed".into()));
    }

    // ── QueueFacade ──────────────────────────────────────────────

    #[test]
    fn queue_facade_basic() {
        let queue = Arc::new(MemoryQueue::new());
        let facade = QueueFacade::new(queue);
        let job = SendEmailJob {
            to: "t@t.com".into(),
            subject: "T".into(),
        };

        let metadata = JobMetadata::new(&job).expect("metadata");
        let id = facade.push(metadata).expect("push");
        assert!(!id.is_empty());
    }

    // ── Dispatch Helpers ─────────────────────────────────────────

    #[test]
    fn dispatch_helper() {
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let job = SendEmailJob {
            to: "dispatch@test.com".into(),
            subject: "Dispatch".into(),
        };

        let id = rf_queue::dispatch(queue.clone(), job).expect("dispatch");
        assert!(!id.is_empty());
    }

    #[test]
    fn dispatch_later_helper() {
        let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
        let job = SendEmailJob {
            to: "later@test.com".into(),
            subject: "Later".into(),
        };

        let id = rf_queue::dispatch_later(queue, job, Duration::from_secs(300))
            .expect("dispatch_later");
        assert!(!id.is_empty());
    }

    // ── QueueConfig ──────────────────────────────────────────────

    #[tokio::test]
    async fn queue_config_memory() {
        let queue = QueueConfig::memory().build().await.expect("build");
        let size = queue.size("default").await.expect("size");
        assert_eq!(size, 0);
    }
}
