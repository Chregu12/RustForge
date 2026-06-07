//! Comprehensive tests for rf-queue

#[cfg(test)]
mod queue_tests {
    use crate::{
        dispatch, dispatch_later, Job, JobMetadata, MemoryQueue, Queue, QueueError, QueueFacade,
    };
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use std::time::Duration;

    // ─── Shared test job definitions ───────────────────────────────────────────

    #[derive(Serialize, Deserialize, Clone)]
    struct SimpleJob {
        message: String,
    }

    #[async_trait]
    impl Job for SimpleJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }
        fn job_type(&self) -> &'static str {
            "simple_job"
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct FailingJob {
        reason: String,
    }

    #[async_trait]
    impl Job for FailingJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Err(QueueError::JobFailed(self.reason.clone()))
        }
        fn job_type(&self) -> &'static str {
            "failing_job"
        }
        fn max_retries(&self) -> u32 {
            2
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct PriorityJob {
        priority: i32,
    }

    #[async_trait]
    impl Job for PriorityJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }
        fn job_type(&self) -> &'static str {
            "priority_job"
        }
        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct CustomQueueJob;

    #[async_trait]
    impl Job for CustomQueueJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }
        fn job_type(&self) -> &'static str {
            "custom_queue_job"
        }
        fn queue(&self) -> &str {
            "emails"
        }
    }

    // ─── Dispatch ──────────────────────────────────────────────────────────────

    #[test]
    fn dispatch_pushes_job_to_default_queue() {
        let queue = Arc::new(MemoryQueue::new());
        let job = SimpleJob { message: "hello".into() };
        let result = dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job);
        assert!(result.is_ok());

        let facade = QueueFacade::new(queue);
        assert_eq!(facade.size("default").unwrap(), 1);
    }

    #[test]
    fn dispatch_returns_job_id() {
        let queue = Arc::new(MemoryQueue::new());
        let job = SimpleJob { message: "id test".into() };
        let id = dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job).unwrap();
        assert!(!id.is_empty());
    }

    #[test]
    fn dispatch_to_custom_named_queue() {
        let queue = Arc::new(MemoryQueue::new());
        let job = CustomQueueJob;
        dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job).unwrap();

        let facade = QueueFacade::new(queue);
        assert_eq!(facade.size("emails").unwrap(), 1);
        assert_eq!(facade.size("default").unwrap(), 0);
    }

    // ─── Reserve ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn reserve_returns_none_on_empty_queue() {
        let queue = MemoryQueue::new();
        let result = queue.reserve("default").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn reserve_returns_job_after_push() {
        let queue = MemoryQueue::new();
        let job = SimpleJob { message: "reserve me".into() };
        let meta = JobMetadata::new(&job).unwrap();
        queue.push(meta).await.unwrap();

        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());
        assert_eq!(reserved.unwrap().job_type, "simple_job");
    }

    #[tokio::test]
    async fn reserve_marks_attempt() {
        let queue = MemoryQueue::new();
        let job = SimpleJob { message: "attempt".into() };
        let meta = JobMetadata::new(&job).unwrap();
        queue.push(meta).await.unwrap();

        let reserved = queue.reserve("default").await.unwrap().unwrap();
        assert_eq!(reserved.attempts, 1);
    }

    // ─── Retry ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn retry_requeues_job_when_attempts_below_max() {
        let queue = MemoryQueue::new();
        let job = FailingJob { reason: "oops".into() };
        let mut meta = JobMetadata::new(&job).unwrap();
        meta.mark_attempt(); // 1 attempt out of 2 max

        queue.retry(meta).await.unwrap();

        assert_eq!(queue.size("default").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn retry_fails_when_max_retries_exceeded() {
        let queue = MemoryQueue::new();
        let job = FailingJob { reason: "exhausted".into() };
        let mut meta = JobMetadata::new(&job).unwrap();
        // Mark 2 attempts (== max_retries), so can_retry() is false
        meta.mark_attempt();
        meta.mark_attempt();

        let result = queue.retry(meta).await;
        assert!(result.is_err());
    }

    #[test]
    fn job_metadata_can_retry_respects_max_retries() {
        let job = FailingJob { reason: "test".into() };
        let mut meta = JobMetadata::new(&job).unwrap();
        assert!(meta.can_retry());

        meta.mark_attempt();
        assert!(meta.can_retry()); // 1 < 2

        meta.mark_attempt();
        assert!(!meta.can_retry()); // 2 == 2
    }

    // ─── Delayed dispatch ──────────────────────────────────────────────────────

    #[test]
    fn delayed_job_is_not_immediately_available() {
        let queue = Arc::new(MemoryQueue::new());
        let job = SimpleJob { message: "later".into() };
        dispatch_later(
            Arc::clone(&queue) as Arc<dyn Queue>,
            job,
            Duration::from_secs(300),
        )
        .unwrap();

        // Queue has the job but it's delayed, so reserve returns None
        let facade = QueueFacade::new(queue);
        // Size still counts it even if not executable
        assert_eq!(facade.size("default").unwrap(), 1);
        // reserve returns None because execute_at is in the future
        let reserved = facade.reserve("default").unwrap();
        assert!(reserved.is_none());
    }

    #[test]
    fn non_delayed_job_is_immediately_available() {
        let queue = Arc::new(MemoryQueue::new());
        let job = SimpleJob { message: "now".into() };
        dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job).unwrap();

        let facade = QueueFacade::new(queue);
        let reserved = facade.reserve("default").unwrap();
        assert!(reserved.is_some());
    }

    // ─── Clear ─────────────────────────────────────────────────────────────────

    #[test]
    fn clear_empties_queue() {
        let queue = Arc::new(MemoryQueue::new());
        let facade = QueueFacade::new(Arc::clone(&queue) as Arc<dyn Queue>);

        for _ in 0..3 {
            let job = SimpleJob { message: "fill".into() };
            dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job).unwrap();
        }
        assert_eq!(facade.size("default").unwrap(), 3);

        facade.clear("default").unwrap();
        assert_eq!(facade.size("default").unwrap(), 0);
    }

    // ─── Priority ──────────────────────────────────────────────────────────────

    #[test]
    fn job_metadata_stores_priority() {
        let job = PriorityJob { priority: 10 };
        let meta = JobMetadata::new(&job).unwrap();
        assert_eq!(meta.priority, 10);
    }

    // ─── Serialization ─────────────────────────────────────────────────────────

    #[test]
    fn job_metadata_round_trips_via_bytes() {
        let job = SimpleJob { message: "serialise me".into() };
        let meta = JobMetadata::new(&job).unwrap();
        let bytes = meta.to_bytes().unwrap();
        let decoded = JobMetadata::from_bytes(&bytes).unwrap();
        assert_eq!(meta.id, decoded.id);
        assert_eq!(meta.job_type, decoded.job_type);
    }

    #[test]
    fn job_metadata_deserialises_payload() {
        let job = SimpleJob { message: "payload".into() };
        let meta = JobMetadata::new(&job).unwrap();
        let decoded: SimpleJob = meta.deserialize().unwrap();
        assert_eq!(decoded.message, "payload");
    }

    // ─── Queue Facade helpers ──────────────────────────────────────────────────

    #[test]
    fn queue_facade_complete_does_not_err() {
        let queue = Arc::new(MemoryQueue::new());
        let job = SimpleJob { message: "complete".into() };
        let id = dispatch(Arc::clone(&queue) as Arc<dyn Queue>, job).unwrap();

        let facade = QueueFacade::new(queue);
        assert!(facade.complete(&id).is_ok());
    }

    #[test]
    fn queue_facade_fail_does_not_err() {
        let queue = Arc::new(MemoryQueue::new());
        let facade = QueueFacade::new(queue);
        assert!(facade.fail("fake-id", "some error").is_ok());
    }

    // ─── Mark error ────────────────────────────────────────────────────────────

    #[test]
    fn job_metadata_mark_error_stores_message() {
        let job = SimpleJob { message: "err".into() };
        let mut meta = JobMetadata::new(&job).unwrap();
        assert!(meta.last_error.is_none());
        meta.mark_error("something went wrong".into());
        assert_eq!(meta.last_error.as_deref(), Some("something went wrong"));
    }
}
