use async_trait::async_trait;
use crate::job::Job;
use crate::error::QueueResult;

pub mod memory;
pub mod redis;

pub use memory::MemoryBackend;
pub use redis::RedisBackend;

/// Queue backend trait
#[async_trait]
pub trait QueueBackend: Send + Sync {
    /// Push a job onto the queue
    async fn push(&self, job: Job) -> QueueResult<()>;

    /// Pop a job from the queue
    async fn pop(&self) -> QueueResult<Option<Job>>;

    /// Pop a job from a specific queue
    async fn pop_from(&self, queue: &str) -> QueueResult<Option<Job>>;

    /// Get the number of jobs in the queue
    async fn size(&self) -> QueueResult<usize>;

    /// Get the number of jobs in a specific queue
    async fn size_of(&self, queue: &str) -> QueueResult<usize>;

    /// Clear all jobs from the queue
    async fn clear(&self) -> QueueResult<()>;

    /// Clear all jobs from a specific queue
    async fn clear_queue(&self, queue: &str) -> QueueResult<()>;

    /// Get a job by ID
    async fn get(&self, job_id: &str) -> QueueResult<Option<Job>>;

    /// Delete a job by ID
    async fn delete(&self, job_id: &str) -> QueueResult<bool>;

    /// Update a job's status
    async fn update(&self, job: &Job) -> QueueResult<()>;

    /// Get all failed jobs
    async fn get_failed(&self) -> QueueResult<Vec<Job>>;

    /// Get all delayed jobs that are ready to execute
    async fn get_ready_delayed(&self) -> QueueResult<Vec<Job>>;

    /// Move a delayed job to the pending queue if it's ready
    async fn release_delayed(&self, job: &Job) -> QueueResult<()>;
}
