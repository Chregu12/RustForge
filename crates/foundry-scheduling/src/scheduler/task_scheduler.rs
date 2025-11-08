use crate::jobs::{JobContext, JobRunner, RunnerConfig, ScheduledJob, JobState as InternalJobState};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::info;

/// High-level task scheduler
pub struct TaskScheduler {
    runner: Arc<JobRunner>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self::with_config(RunnerConfig::default())
    }

    pub fn with_config(config: RunnerConfig) -> Self {
        Self {
            runner: Arc::new(JobRunner::new(config)),
        }
    }

    /// Schedule a job
    pub async fn schedule(&self, job: Arc<dyn ScheduledJob>) {
        self.runner.register(job).await;
    }

    /// Schedule a closure-based job
    pub async fn schedule_fn<F, Fut>(
        &self,
        name: impl Into<String>,
        schedule: impl Into<String>,
        func: F,
    ) where
        F: Fn(JobContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), super::super::jobs::JobError>> + Send + 'static,
    {
        use crate::jobs::scheduled_job::FunctionJob;

        let job = FunctionJob::new(name, schedule, move |ctx| Box::pin(func(ctx)));
        self.runner.register(Arc::new(job)).await;
    }

    /// Start the scheduler (blocking)
    pub async fn run(&self) {
        info!("Starting task scheduler");
        self.runner.start().await;
    }

    /// Get all scheduled job names
    pub async fn list_jobs(&self) -> Vec<String> {
        self.runner.list().await
    }

    /// Get job states
    pub async fn job_states(&self) -> std::collections::HashMap<String, JobState> {
        let states = self.runner.states().await;
        states
            .into_iter()
            .map(|(name, state)| (name, JobState::from(state)))
            .collect()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Job state for external visibility
#[derive(Debug, Clone)]
pub struct JobState {
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub is_running: bool,
}

impl From<InternalJobState> for JobState {
    fn from(state: InternalJobState) -> Self {
        Self {
            last_run: state.last_run,
            next_run: state.next_run,
            is_running: state.is_running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = TaskScheduler::new();
        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_schedule_fn() {
        let scheduler = TaskScheduler::new();

        scheduler
            .schedule_fn("test_job", "* * * * *", |_ctx| async {
                Ok(())
            })
            .await;

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 1);
    }
}
