use super::{JobContext, JobError, ScheduledJob};
use crate::cron::CronSchedule;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

/// Job runner configuration
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub check_interval: TokioDuration,
    pub timezone: chrono_tz::Tz,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            check_interval: TokioDuration::from_secs(60),
            timezone: chrono_tz::UTC,
        }
    }
}

/// Job execution state
#[derive(Debug, Clone)]
pub struct JobState {
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub is_running: bool,
}

/// Job runner for executing scheduled jobs
pub struct JobRunner {
    jobs: Arc<RwLock<HashMap<String, Arc<dyn ScheduledJob>>>>,
    states: Arc<RwLock<HashMap<String, JobState>>>,
    config: RunnerConfig,
}

impl JobRunner {
    pub fn new(config: RunnerConfig) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a scheduled job
    pub async fn register(&self, job: Arc<dyn ScheduledJob>) {
        let name = job.name().to_string();
        let schedule_expr = job.schedule();

        // Calculate next run time
        let schedule = match CronSchedule::with_timezone(schedule_expr, self.config.timezone) {
            Ok(s) => s,
            Err(e) => {
                error!(job = %name, error = %e, "Invalid cron schedule");
                return;
            }
        };

        let next_run = schedule.next();

        let state = JobState {
            last_run: None,
            next_run,
            is_running: false,
        };

        let mut jobs = self.jobs.write().await;
        let mut states = self.states.write().await;

        jobs.insert(name.clone(), job);
        states.insert(name.clone(), state);

        info!(job = %name, next_run = ?next_run, "Scheduled job registered");
    }

    /// Start the job runner
    pub async fn start(&self) {
        info!("Starting job runner");
        let mut ticker = interval(self.config.check_interval);

        loop {
            ticker.tick().await;
            self.check_and_run_jobs().await;
        }
    }

    /// Check for due jobs and run them
    async fn check_and_run_jobs(&self) {
        let now = Utc::now();
        let jobs = self.jobs.read().await;
        let mut states = self.states.write().await;

        for (name, job) in jobs.iter() {
            if let Some(state) = states.get_mut(name) {
                // Check if job is due
                if let Some(next_run) = state.next_run {
                    if next_run <= now {
                        // Check if overlapping is allowed
                        if state.is_running && !job.allow_overlapping() {
                            warn!(job = %name, "Job still running, skipping");
                            continue;
                        }

                        // Mark as running
                        state.is_running = true;
                        state.last_run = Some(now);

                        // Calculate next run time
                        let schedule_expr = job.schedule();
                        if let Ok(schedule) = CronSchedule::with_timezone(schedule_expr, self.config.timezone) {
                            state.next_run = schedule.next_after(&now);
                        }

                        // Spawn job execution
                        let job = job.clone();
                        let name = name.clone();
                        let states = self.states.clone();

                        tokio::spawn(async move {
                            Self::execute_job(job, name, states, next_run).await;
                        });
                    }
                }
            }
        }
    }

    /// Execute a single job
    async fn execute_job(
        job: Arc<dyn ScheduledJob>,
        name: String,
        states: Arc<RwLock<HashMap<String, JobState>>>,
        scheduled_at: DateTime<Utc>,
    ) {
        debug!(job = %name, "Executing job");

        let context = JobContext::new(scheduled_at);

        let result = if let Some(timeout) = job.timeout() {
            tokio::select! {
                res = job.execute(context.clone()) => res,
                _ = sleep(timeout) => Err(JobError::Timeout),
            }
        } else {
            job.execute(context.clone()).await
        };

        match result {
            Ok(_) => {
                info!(job = %name, duration = ?(Utc::now() - context.started_at), "Job completed successfully");
                job.on_success().await;
            }
            Err(ref e) => {
                error!(job = %name, error = %e, "Job failed");
                job.on_failure(e).await;
            }
        }

        // Mark as not running
        let mut states = states.write().await;
        if let Some(state) = states.get_mut(&name) {
            state.is_running = false;
        }
    }

    /// Get job states
    pub async fn states(&self) -> HashMap<String, JobState> {
        self.states.read().await.clone()
    }

    /// List all registered jobs
    pub async fn list(&self) -> Vec<String> {
        self.jobs.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::scheduled_job::{JobResult, ScheduledJob};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CounterJob {
        counter: Arc<AtomicU32>,
    }

    #[async_trait]
    impl ScheduledJob for CounterJob {
        fn name(&self) -> &str {
            "counter"
        }

        fn schedule(&self) -> &str {
            "* * * * *"
        }

        async fn execute(&self, _context: JobContext) -> JobResult {
            self.counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_job_registration() {
        let runner = JobRunner::new(RunnerConfig::default());
        let counter = Arc::new(AtomicU32::new(0));
        let job = Arc::new(CounterJob { counter });

        runner.register(job).await;

        let jobs = runner.list().await;
        assert_eq!(jobs.len(), 1);
        assert!(jobs.contains(&"counter".to_string()));
    }
}
