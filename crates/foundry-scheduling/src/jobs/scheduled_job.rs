use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Scheduled job trait
#[async_trait]
pub trait ScheduledJob: Send + Sync {
    /// Get the job name
    fn name(&self) -> &str;

    /// Get the cron schedule
    fn schedule(&self) -> &str;

    /// Execute the job
    async fn execute(&self, context: JobContext) -> JobResult;

    /// Called when the job fails
    async fn on_failure(&self, _error: &JobError) {}

    /// Called when the job succeeds
    async fn on_success(&self) {}

    /// Whether the job should run even if the previous run hasn't finished
    fn allow_overlapping(&self) -> bool {
        false
    }

    /// Maximum execution time (None = no limit)
    fn timeout(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(300)) // 5 minutes default
    }
}

/// Job execution context
#[derive(Debug, Clone)]
pub struct JobContext {
    pub job_id: String,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: DateTime<Utc>,
    pub attempt: u32,
}

impl JobContext {
    pub fn new(scheduled_at: DateTime<Utc>) -> Self {
        Self {
            job_id: Uuid::new_v4().to_string(),
            scheduled_at,
            started_at: Utc::now(),
            attempt: 1,
        }
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt = attempt;
        self
    }
}

/// Job execution result
pub type JobResult = Result<(), JobError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobError {
    ExecutionFailed(String),
    Timeout,
    Cancelled,
    Other(String),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            JobError::Timeout => write!(f, "Job timed out"),
            JobError::Cancelled => write!(f, "Job cancelled"),
            JobError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for JobError {}

/// Simple function-based job
pub struct FunctionJob<F>
where
    F: Fn(JobContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = JobResult> + Send>> + Send + Sync,
{
    name: String,
    schedule: String,
    func: F,
}

impl<F> FunctionJob<F>
where
    F: Fn(JobContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = JobResult> + Send>> + Send + Sync,
{
    pub fn new(name: impl Into<String>, schedule: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            schedule: schedule.into(),
            func,
        }
    }
}

#[async_trait]
impl<F> ScheduledJob for FunctionJob<F>
where
    F: Fn(JobContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = JobResult> + Send>> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn schedule(&self) -> &str {
        &self.schedule
    }

    async fn execute(&self, context: JobContext) -> JobResult {
        (self.func)(context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestJob;

    #[async_trait]
    impl ScheduledJob for TestJob {
        fn name(&self) -> &str {
            "test_job"
        }

        fn schedule(&self) -> &str {
            "* * * * *"
        }

        async fn execute(&self, _context: JobContext) -> JobResult {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_scheduled_job() {
        let job = TestJob;
        let context = JobContext::new(Utc::now());
        let result = job.execute(context).await;
        assert!(result.is_ok());
    }
}
