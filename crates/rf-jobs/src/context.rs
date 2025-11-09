//! Job execution context

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Context provided to jobs during execution
#[derive(Debug, Clone)]
pub struct JobContext {
    /// Unique job ID
    job_id: Uuid,

    /// Queue name
    queue: String,

    /// Current attempt number (1-indexed)
    attempt: u32,

    /// Maximum retry attempts
    max_attempts: u32,

    /// When job was dispatched
    dispatched_at: DateTime<Utc>,

    /// When job started executing
    started_at: DateTime<Utc>,
}

impl JobContext {
    /// Create a new job context
    pub fn new(
        job_id: Uuid,
        queue: String,
        attempt: u32,
        max_attempts: u32,
        dispatched_at: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            queue,
            attempt,
            max_attempts,
            dispatched_at,
            started_at: Utc::now(),
        }
    }

    /// Get unique job ID
    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// Get current attempt number (1-indexed)
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Get maximum attempts allowed
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Check if this is the final attempt
    pub fn is_final_attempt(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    /// Get queue name
    pub fn queue(&self) -> &str {
        &self.queue
    }

    /// Get when job was dispatched
    pub fn dispatched_at(&self) -> DateTime<Utc> {
        self.dispatched_at
    }

    /// Get when job started executing
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Log message with job context
    pub fn log(&self, message: &str) {
        tracing::info!(
            job_id = %self.job_id,
            queue = %self.queue,
            attempt = self.attempt,
            max_attempts = self.max_attempts,
            "{}",
            message
        );
    }

    /// Log warning with job context
    pub fn warn(&self, message: &str) {
        tracing::warn!(
            job_id = %self.job_id,
            queue = %self.queue,
            attempt = self.attempt,
            "{}",
            message
        );
    }

    /// Log error with job context
    pub fn error(&self, message: &str) {
        tracing::error!(
            job_id = %self.job_id,
            queue = %self.queue,
            attempt = self.attempt,
            "{}",
            message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_context_creation() {
        let job_id = Uuid::new_v4();
        let ctx = JobContext::new(
            job_id,
            "default".to_string(),
            1,
            3,
            Utc::now(),
        );

        assert_eq!(ctx.job_id(), job_id);
        assert_eq!(ctx.queue(), "default");
        assert_eq!(ctx.attempt(), 1);
        assert_eq!(ctx.max_attempts(), 3);
        assert!(!ctx.is_final_attempt());
    }

    #[test]
    fn test_final_attempt() {
        let ctx = JobContext::new(
            Uuid::new_v4(),
            "default".to_string(),
            3,
            3,
            Utc::now(),
        );

        assert!(ctx.is_final_attempt());
    }
}
