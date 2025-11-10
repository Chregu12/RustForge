//! Queue trait and implementations

use crate::error::QueueResult;
use crate::job::JobMetadata;
use async_trait::async_trait;

/// Queue backend trait
#[async_trait]
pub trait Queue: Send + Sync {
    /// Push a job to the queue
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String>;

    /// Reserve the next job for processing
    async fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>>;

    /// Mark a job as completed
    async fn complete(&self, job_id: &str) -> QueueResult<()>;

    /// Mark a job as failed
    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()>;

    /// Retry a failed job
    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()>;

    /// Get job count for a queue
    async fn size(&self, queue: &str) -> QueueResult<usize>;

    /// Clear a queue
    async fn clear(&self, queue: &str) -> QueueResult<()>;
}
