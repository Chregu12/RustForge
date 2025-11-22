//! AWS SQS queue backend driver
//!
//! Provides an AWS SQS-backed queue solution.

use crate::{JobMetadata, Queue, QueueError, QueueResult};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_sqs::{Client, Error};
use std::collections::HashMap;

/// SQS queue driver
pub struct SqsQueue {
    client: Client,
    queue_url: String,
    /// Map to store message receipt handles for deletion
    receipt_handles: std::sync::Arc<tokio::sync::RwLock<HashMap<String, String>>>,
}

impl SqsQueue {
    /// Create a new SQS queue driver
    ///
    /// # Arguments
    ///
    /// * `queue_url` - Full URL of the SQS queue
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::drivers::sqs::SqsQueue;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let queue = SqsQueue::new(
    ///     "https://sqs.us-east-1.amazonaws.com/123456789012/my-queue".to_string()
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(queue_url: String) -> Result<Self, QueueError> {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            queue_url,
            receipt_handles: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Create a new SQS queue driver with custom region
    pub async fn with_region(queue_url: String, region: String) -> Result<Self, QueueError> {
        let region_provider = aws_config::Region::new(region);
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            queue_url,
            receipt_handles: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        })
    }

    /// Purge all messages from the queue
    pub async fn purge(&self) -> Result<(), QueueError> {
        self.client
            .purge_queue()
            .queue_url(&self.queue_url)
            .send()
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to purge SQS queue: {}", e)))?;

        Ok(())
    }

    /// Get queue attributes
    pub async fn get_attributes(&self) -> Result<HashMap<String, String>, QueueError> {
        let result = self
            .client
            .get_queue_attributes()
            .queue_url(&self.queue_url)
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::All)
            .send()
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to get queue attributes: {}", e)))?;

        Ok(result
            .attributes()
            .map(|attrs| {
                attrs
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.clone()))
                    .collect()
            })
            .unwrap_or_default())
    }
}

#[async_trait]
impl Queue for SqsQueue {
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let payload = serde_json::to_string(&metadata)
            .map_err(|e| QueueError::SerializationError(e.to_string()))?;

        let mut request = self
            .client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(payload);

        // Set delay if specified (calculate from execute_at)
        if let Some(execute_at) = metadata.execute_at {
            let now = chrono::Utc::now();
            if execute_at > now {
                let delay_secs = (execute_at - now).num_seconds();
                if delay_secs > 0 && delay_secs <= 900 {
                    // SQS max delay is 900 seconds (15 minutes)
                    request = request.delay_seconds(delay_secs as i32);
                }
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to send SQS message: {}", e)))?;

        Ok(response.message_id().unwrap_or_default().to_string())
    }

    async fn reserve(&self, _queue: &str) -> QueueResult<Option<JobMetadata>> {
        let response = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(1)
            .wait_time_seconds(5) // Long polling
            .visibility_timeout(30) // 30 seconds to process
            .send()
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to receive SQS message: {}", e)))?;

        if let Some(message) = response.messages().first() {
            let body = message.body().unwrap_or("");
            let mut metadata: JobMetadata = serde_json::from_str(body)
                .map_err(|e| QueueError::DeserializationError(e.to_string()))?;

            // Generate a unique job ID
            let job_id = uuid::Uuid::new_v4().to_string();
            metadata.id = job_id.clone();

            // Store receipt handle for later deletion
            if let Some(receipt_handle) = message.receipt_handle() {
                let mut handles = self.receipt_handles.write().await;
                handles.insert(job_id, receipt_handle.to_string());
            }

            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        // Get receipt handle
        let receipt_handle = {
            let mut handles = self.receipt_handles.write().await;
            handles.remove(job_id).ok_or_else(|| {
                QueueError::JobNotFound(format!("Receipt handle not found for job {}", job_id))
            })?
        };

        // Delete message
        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(&receipt_handle)
            .send()
            .await
            .map_err(|e| QueueError::Backend(format!("Failed to delete SQS message: {}", e)))?;

        Ok(())
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        // For SQS, we log the error and delete the message
        // In production, you might want to send to a dead-letter queue
        tracing::error!("Job {} failed: {}", job_id, error);

        // Delete the message (or let it become visible again)
        self.complete(job_id).await
    }

    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        // Push back to queue with delay
        let mut retry_metadata = metadata.clone();
        retry_metadata.execute_at = Some(chrono::Utc::now() + chrono::Duration::seconds(60)); // 1 minute delay

        self.push(retry_metadata).await?;
        Ok(())
    }

    async fn size(&self, _queue: &str) -> QueueResult<usize> {
        let attributes = self.get_attributes().await?;

        let size = attributes
            .get("ApproximateNumberOfMessages")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        Ok(size)
    }

    async fn clear(&self, _queue: &str) -> QueueResult<()> {
        self.purge().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require AWS credentials and an SQS queue
    // They are ignored by default

    #[tokio::test]
    #[ignore]
    async fn test_sqs_queue_operations() {
        use crate::Job;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, Clone)]
        struct TestJob {
            data: String,
        }

        #[async_trait]
        impl Job for TestJob {
            async fn handle(&self) -> Result<(), QueueError> {
                Ok(())
            }

            fn job_type(&self) -> &'static str {
                "test_job"
            }
        }

        let queue_url = std::env::var("TEST_SQS_QUEUE_URL")
            .expect("TEST_SQS_QUEUE_URL environment variable not set");

        let queue = SqsQueue::new(queue_url).await.unwrap();

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        // Push job
        let job_id = queue.push(metadata).await.unwrap();
        assert!(!job_id.is_empty());

        // Reserve job
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());

        // Complete job
        if let Some(meta) = reserved {
            queue.complete(&meta.id.unwrap()).await.unwrap();
        }
    }
}
