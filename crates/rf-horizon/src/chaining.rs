//! Job chaining for sequential execution

use crate::batching::Job;
use anyhow::Result;
use std::sync::Arc;

/// Chain of jobs to execute sequentially
pub struct Chain {
    jobs: Vec<Arc<dyn Job>>,
}

impl Chain {
    /// Create a new job chain
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Add a job to the chain
    pub fn job(mut self, job: Arc<dyn Job>) -> Self {
        self.jobs.push(job);
        self
    }

    /// Add a job and continue chaining (alias for job)
    pub fn then(self, job: Arc<dyn Job>) -> Self {
        self.job(job)
    }

    /// Dispatch the chain for execution
    pub async fn dispatch(self) -> Result<ChainHandle> {
        let jobs = self.jobs;

        let handle = ChainHandle {
            total_jobs: jobs.len(),
            completed_jobs: Arc::new(tokio::sync::RwLock::new(0)),
        };

        let completed_clone = Arc::clone(&handle.completed_jobs);

        tokio::spawn(async move {
            for job in jobs {
                match job.handle().await {
                    Ok(_) => {
                        let mut completed = completed_clone.write().await;
                        *completed += 1;
                    }
                    Err(e) => {
                        eprintln!("Chain job '{}' failed: {}", job.name(), e);
                        // Stop chain on first failure
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Get number of jobs in chain
    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

impl Default for Chain {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a dispatched chain for querying status
pub struct ChainHandle {
    total_jobs: usize,
    completed_jobs: Arc<tokio::sync::RwLock<usize>>,
}

impl ChainHandle {
    /// Get number of completed jobs
    pub async fn completed(&self) -> usize {
        *self.completed_jobs.read().await
    }

    /// Get total number of jobs
    pub fn total(&self) -> usize {
        self.total_jobs
    }

    /// Check if chain is finished
    pub async fn is_finished(&self) -> bool {
        self.completed().await >= self.total_jobs
    }

    /// Wait for chain to complete
    pub async fn wait(&self) {
        loop {
            if self.is_finished().await {
                return;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestJob {
        name: String,
        should_fail: bool,
        delay_ms: u64,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<()> {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
            if self.should_fail {
                anyhow::bail!("Job failed");
            }
            Ok(())
        }

        fn name(&self) -> String {
            self.name.clone()
        }
    }

    #[tokio::test]
    async fn test_chain_creation() {
        let chain = Chain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_chain_execution() {
        let chain = Chain::new()
            .job(Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
                delay_ms: 10,
            }))
            .then(Arc::new(TestJob {
                name: "job2".to_string(),
                should_fail: false,
                delay_ms: 10,
            }))
            .then(Arc::new(TestJob {
                name: "job3".to_string(),
                should_fail: false,
                delay_ms: 10,
            }));

        assert_eq!(chain.len(), 3);

        let handle = chain.dispatch().await.unwrap();
        handle.wait().await;

        assert_eq!(handle.completed().await, 3);
        assert!(handle.is_finished().await);
    }

    #[tokio::test]
    async fn test_chain_stops_on_failure() {
        let chain = Chain::new()
            .job(Arc::new(TestJob {
                name: "job1".to_string(),
                should_fail: false,
                delay_ms: 10,
            }))
            .then(Arc::new(TestJob {
                name: "job2".to_string(),
                should_fail: true,
                delay_ms: 10,
            }))
            .then(Arc::new(TestJob {
                name: "job3".to_string(),
                should_fail: false,
                delay_ms: 10,
            }));

        let handle = chain.dispatch().await.unwrap();

        // Wait for execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Should have completed only 1 job (stops at failure)
        assert_eq!(handle.completed().await, 1);
    }
}
