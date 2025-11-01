use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use foundry_plugins::{CommandError, QueueJob, QueuePort};
use tracing::info;

#[derive(Clone, Default)]
pub struct InMemoryQueue {
    jobs: Arc<Mutex<Vec<QueueJob>>>,
}

impl InMemoryQueue {
    pub fn jobs(&self) -> Vec<QueueJob> {
        self.jobs.lock().unwrap().clone()
    }
}

#[async_trait]
impl QueuePort for InMemoryQueue {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError> {
        info!(name = %job.name, "Queue job dispatched");
        self.jobs.lock().unwrap().push(job);
        Ok(())
    }
}
