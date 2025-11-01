use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand, QueueJob};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct QueueWorkCommand {
    descriptor: CommandDescriptor,
}

impl Default for QueueWorkCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueWorkCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.queue_work", "queue:work")
            .summary("Startet den Queue Worker")
            .description("Verarbeitet Jobs aus der Queue mit konfigurierbarem Timeout.")
            .category(CommandKind::Runtime)
            .build();

        Self { descriptor }
    }

    fn parse_timeout(args: &[String]) -> u64 {
        for arg in args {
            if arg.starts_with("--timeout=") {
                if let Some(value) = arg.strip_prefix("--timeout=") {
                    if let Ok(timeout) = value.parse::<u64>() {
                        return timeout;
                    }
                }
            }
        }
        300 // Default: 5 minutes
    }

    #[allow(dead_code)]
    async fn process_job(job: &QueueJob) -> Result<(), CommandError> {
        info!(
            job_name = %job.name,
            payload = ?job.payload,
            "Processing queue job"
        );

        // Simulate job processing
        // In a real implementation, this would:
        // 1. Load the job handler from app/jobs/
        // 2. Execute the handler with the payload
        // 3. Handle success/failure/retry logic

        match job.name.as_str() {
            "send_email" => {
                info!("Sending email: {:?}", job.payload);
                sleep(Duration::from_millis(100)).await;
                Ok(())
            }
            "process_upload" => {
                info!("Processing upload: {:?}", job.payload);
                sleep(Duration::from_millis(200)).await;
                Ok(())
            }
            "generate_report" => {
                info!("Generating report: {:?}", job.payload);
                sleep(Duration::from_millis(500)).await;
                Ok(())
            }
            _ => {
                warn!(job_name = %job.name, "Unknown job type");
                Err(CommandError::Message(format!(
                    "Unknown job type: {}",
                    job.name
                )))
            }
        }
    }
}

#[async_trait]
impl FoundryCommand for QueueWorkCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let timeout_seconds = Self::parse_timeout(&ctx.args);
        let timeout_duration = Duration::from_secs(timeout_seconds);

        info!(
            timeout_seconds = timeout_seconds,
            "Queue worker starting"
        );

        let start_time = std::time::Instant::now();
        let jobs_processed = 0;
        let jobs_failed = 0;

        // In a real implementation, this would be a proper queue polling mechanism
        // For now, we simulate by checking if the queue has jobs
        loop {
            if start_time.elapsed() >= timeout_duration {
                info!("Queue worker timeout reached, shutting down");
                break;
            }

            // Check for shutdown signal (Ctrl+C)
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Received shutdown signal, stopping worker");
                    break;
                }
                _ = sleep(Duration::from_secs(1)) => {
                    // In a real implementation, we would poll the queue here
                    // For now, we just simulate with a short sleep

                    // This is where we would get jobs from the queue
                    // let jobs = ctx.queue.fetch_pending_jobs().await?;

                    // For demonstration, we'll just break after timeout
                    if start_time.elapsed() >= Duration::from_secs(5) && jobs_processed == 0 {
                        info!("No jobs found in queue after 5 seconds");
                        break;
                    }
                }
            }
        }

        let elapsed = start_time.elapsed();
        let message = format!(
            "Queue worker stopped after {:.2}s. Processed: {}, Failed: {}",
            elapsed.as_secs_f64(),
            jobs_processed,
            jobs_failed
        );

        let data = json!({
            "duration_seconds": elapsed.as_secs(),
            "jobs_processed": jobs_processed,
            "jobs_failed": jobs_failed,
            "timeout_seconds": timeout_seconds,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::ResponseFormat;
    use serde_json::Value;

    fn create_test_context(args: Vec<String>) -> CommandContext {
        CommandContext {
            args,
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config: Value::Null,
            options: Default::default(),
            artifacts: std::sync::Arc::new(foundry_infra::LocalArtifactPort::default()),
            migrations: std::sync::Arc::new(foundry_infra::SeaOrmMigrationService::default()),
            seeds: std::sync::Arc::new(foundry_infra::SeaOrmSeedService::default()),
            validation: std::sync::Arc::new(foundry_infra::SimpleValidationService::default()),
            storage: std::sync::Arc::new(foundry_infra::FileStorageAdapter::new(
                std::sync::Arc::new(foundry_storage::manager::StorageManager::new(
                    foundry_storage::config::StorageConfig::from_env()
                ).unwrap())
            )),
            cache: std::sync::Arc::new(foundry_infra::InMemoryCacheStore::default()),
            queue: std::sync::Arc::new(foundry_infra::InMemoryQueue::default()),
            events: std::sync::Arc::new(foundry_infra::InMemoryEventBus::default()),
        }
    }

    #[tokio::test]
    async fn test_queue_work_default_timeout() {
        let command = QueueWorkCommand::new();
        let ctx = create_test_context(vec![]);

        // This will timeout after ~5 seconds since there are no jobs
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            command.execute(ctx)
        ).await;

        assert!(result.is_ok());
        let command_result = result.unwrap().unwrap();
        assert_eq!(command_result.status, CommandStatus::Success);
    }

    #[tokio::test]
    async fn test_queue_work_custom_timeout() {
        let command = QueueWorkCommand::new();
        let ctx = create_test_context(vec!["--timeout=2".to_string()]);

        let result = tokio::time::timeout(
            Duration::from_secs(10),
            command.execute(ctx)
        ).await;

        assert!(result.is_ok());
        let command_result = result.unwrap().unwrap();
        assert_eq!(command_result.status, CommandStatus::Success);

        let data = command_result.data.unwrap();
        assert_eq!(data["timeout_seconds"], 2);
    }

    #[tokio::test]
    async fn test_queue_work_parse_timeout() {
        assert_eq!(QueueWorkCommand::parse_timeout(&[]), 300);
        assert_eq!(
            QueueWorkCommand::parse_timeout(&["--timeout=60".to_string()]),
            60
        );
        assert_eq!(
            QueueWorkCommand::parse_timeout(&["--timeout=invalid".to_string()]),
            300
        );
    }

    #[tokio::test]
    async fn test_queue_work_descriptor() {
        let command = QueueWorkCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "queue:work");
        assert_eq!(descriptor.category, CommandKind::Runtime);
    }

    #[tokio::test]
    async fn test_process_job_send_email() {
        let job = QueueJob {
            name: "send_email".to_string(),
            payload: json!({"to": "test@example.com"}),
            delay_seconds: None,
        };

        let result = QueueWorkCommand::process_job(&job).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_job_unknown() {
        let job = QueueJob {
            name: "unknown_job".to_string(),
            payload: json!({}),
            delay_seconds: None,
        };

        let result = QueueWorkCommand::process_job(&job).await;
        assert!(result.is_err());
    }
}
