//! Retry logic configuration

use std::time::Duration;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub delay: Duration,
    pub backoff_multiplier: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_backoff(mut self, multiplier: f32) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            delay: Duration::from_secs(0),
            backoff_multiplier: 1.0,
        }
    }
}

/// Retry policy trait
pub trait RetryPolicy: Send + Sync {
    fn should_retry(&self, attempt: u32, error: &reqwest::Error) -> bool;
    fn delay(&self, attempt: u32) -> Duration;
}

/// Default retry policy
pub struct DefaultRetryPolicy {
    config: RetryConfig,
}

impl DefaultRetryPolicy {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
}

impl RetryPolicy for DefaultRetryPolicy {
    fn should_retry(&self, attempt: u32, error: &reqwest::Error) -> bool {
        if attempt >= self.config.max_retries {
            return false;
        }

        // Retry on network errors or 5xx server errors
        error.is_timeout() || error.is_connect() || error.status().map_or(false, |s| s.is_server_error())
    }

    fn delay(&self, attempt: u32) -> Duration {
        let multiplier = self.config.backoff_multiplier.powi(attempt as i32);
        Duration::from_secs_f32(self.config.delay.as_secs_f32() * multiplier)
    }
}
