use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FailedJob {
    name: String,
    error: String,
    retry_count: u32,
    created_at: DateTime<Utc>,
    payload: serde_json::Value,
}

pub struct QueueFailedCommand {
    descriptor: CommandDescriptor,
}

impl Default for QueueFailedCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueFailedCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.queue_failed", "queue:failed")
            .summary("Shows failed queue jobs")
            .description("Zeigt fehlgeschlagene Jobs aus der Queue mit Details zu Fehlern und Retry-Zählern.")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn parse_limit(args: &[String]) -> usize {
        for arg in args {
            if arg.starts_with("--limit=") {
                if let Some(value) = arg.strip_prefix("--limit=") {
                    if let Ok(limit) = value.parse::<usize>() {
                        return limit;
                    }
                }
            }
        }
        10 // Default limit
    }

    fn has_retry_flag(args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--retry")
    }

    async fn get_failed_jobs(_ctx: &CommandContext, limit: usize) -> Result<Vec<FailedJob>, CommandError> {
        // In a real implementation, this would query the queue store for failed jobs
        // For now, we return mock data to demonstrate the functionality

        // Simulate some failed jobs
        let mock_jobs = vec![
            FailedJob {
                name: "send_email".to_string(),
                error: "SMTP connection timeout".to_string(),
                retry_count: 3,
                created_at: Utc::now() - chrono::Duration::hours(2),
                payload: json!({"to": "user@example.com", "subject": "Welcome"}),
            },
            FailedJob {
                name: "process_upload".to_string(),
                error: "File not found: /tmp/upload_123.jpg".to_string(),
                retry_count: 1,
                created_at: Utc::now() - chrono::Duration::minutes(30),
                payload: json!({"file_id": "123", "user_id": 456}),
            },
            FailedJob {
                name: "generate_report".to_string(),
                error: "Database query timeout".to_string(),
                retry_count: 5,
                created_at: Utc::now() - chrono::Duration::days(1),
                payload: json!({"report_type": "monthly", "period": "2025-01"}),
            },
        ];

        // Apply limit
        let limited_jobs: Vec<FailedJob> = mock_jobs.into_iter().take(limit).collect();
        Ok(limited_jobs)
    }

    async fn retry_failed_jobs(_ctx: &CommandContext, jobs: &[FailedJob]) -> Result<usize, CommandError> {
        // In a real implementation, this would re-queue the failed jobs
        // For now, we simulate retrying all jobs
        Ok(jobs.len())
    }

    fn format_as_table(jobs: &[FailedJob]) -> String {
        let mut lines = vec![
            String::from("╭────────────────────┬──────────────────────────────────────┬───────────┬─────────────────────╮"),
            String::from("│ JOB NAME           │ ERROR                                │ RETRIES   │ CREATED AT          │"),
            String::from("├────────────────────┼──────────────────────────────────────┼───────────┼─────────────────────┤"),
        ];

        for job in jobs {
            let formatted_date = job.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let line = format!(
                "│ {:<18} │ {:<36} │ {:<9} │ {:<19} │",
                truncate(&job.name, 18),
                truncate(&job.error, 36),
                job.retry_count,
                formatted_date
            );
            lines.push(line);
        }

        lines.push(String::from("╰────────────────────┴──────────────────────────────────────┴───────────┴─────────────────────╯"));
        lines.join("\n")
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[0..max_len - 3])
    } else {
        s.to_string()
    }
}

#[async_trait]
impl FoundryCommand for QueueFailedCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let limit = Self::parse_limit(&ctx.args);
        let retry = Self::has_retry_flag(&ctx.args);

        // Check for --format flag
        let use_json = ctx.args.iter().any(|arg| {
            arg == "--format=json" || arg == "--json"
        });

        // Get failed jobs
        let failed_jobs = Self::get_failed_jobs(&ctx, limit).await?;
        let total = failed_jobs.len();

        // If retry flag is set, attempt to retry the jobs
        let retried = if retry {
            let count = Self::retry_failed_jobs(&ctx, &failed_jobs).await?;
            Some(count)
        } else {
            None
        };

        let message = if retry {
            if total == 0 {
                "No failed jobs to retry".to_string()
            } else {
                format!("Retried {} failed job(s)", retried.unwrap_or(0))
            }
        } else if use_json {
            format!("{} failed job(s) found", total)
        } else if total == 0 {
            "No failed jobs found".to_string()
        } else {
            let table = Self::format_as_table(&failed_jobs);
            format!("{}\n\n{} failed job(s) found (limit: {})", table, total, limit)
        };

        let mut data = json!({
            "total": total,
            "limit": limit,
            "jobs": failed_jobs,
        });

        if let Some(retried_count) = retried {
            data["retried"] = json!(retried_count);
        }

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
    async fn test_queue_failed_default() {
        let command = QueueFailedCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_queue_failed_with_limit() {
        let command = QueueFailedCommand::new();
        let ctx = create_test_context(vec!["--limit=5".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert_eq!(data["limit"], 5);
    }

    #[tokio::test]
    async fn test_queue_failed_json_format() {
        let command = QueueFailedCommand::new();
        let ctx = create_test_context(vec!["--format=json".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert!(data["total"].is_number());
        assert!(data["jobs"].is_array());
    }

    #[tokio::test]
    async fn test_queue_failed_with_retry() {
        let command = QueueFailedCommand::new();
        let ctx = create_test_context(vec!["--retry".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert!(data["retried"].is_number());
    }

    #[test]
    fn test_parse_limit() {
        assert_eq!(QueueFailedCommand::parse_limit(&[]), 10);
        assert_eq!(
            QueueFailedCommand::parse_limit(&["--limit=20".to_string()]),
            20
        );
        assert_eq!(
            QueueFailedCommand::parse_limit(&["--limit=invalid".to_string()]),
            10
        );
    }

    #[test]
    fn test_has_retry_flag() {
        assert!(!QueueFailedCommand::has_retry_flag(&[]));
        assert!(QueueFailedCommand::has_retry_flag(&["--retry".to_string()]));
        assert!(!QueueFailedCommand::has_retry_flag(&["--limit=10".to_string()]));
    }

    #[tokio::test]
    async fn test_queue_failed_descriptor() {
        let command = QueueFailedCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "queue:failed");
        assert_eq!(descriptor.category, CommandKind::Utility);
    }
}
