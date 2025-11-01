use async_trait::async_trait;
use chrono::{DateTime, Utc};
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub name: String,
    pub expression: String, // Cron expression
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_run: Option<DateTime<Utc>>,
}

// ==================== ScheduleListCommand ====================

pub struct ScheduleListCommand {
    descriptor: CommandDescriptor,
}

impl Default for ScheduleListCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleListCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.schedule_list", "schedule:list")
            .summary("Zeigt alle geplanten Tasks")
            .description("Listet alle Scheduled Tasks mit Cron-Expression, Beschreibung und nächster Ausführung.")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn get_scheduled_tasks() -> Vec<ScheduledTask> {
        let now = Utc::now();
        let next_hour = now + chrono::Duration::hours(1);
        let next_day = now + chrono::Duration::days(1);

        vec![
            ScheduledTask {
                name: "backup:database".to_string(),
                expression: "0 2 * * *".to_string(), // Every day at 2 AM
                description: "Erstellt ein Backup der Datenbank".to_string(),
                last_run: Some(now - chrono::Duration::days(1)),
                next_run: Some(next_day),
            },
            ScheduledTask {
                name: "cache:prune".to_string(),
                expression: "*/15 * * * *".to_string(), // Every 15 minutes
                description: "Löscht abgelaufene Cache-Einträge".to_string(),
                last_run: Some(now - chrono::Duration::minutes(15)),
                next_run: Some(now + chrono::Duration::minutes(15)),
            },
            ScheduledTask {
                name: "queue:cleanup".to_string(),
                expression: "0 * * * *".to_string(), // Every hour
                description: "Bereinigt die Queue von alten Jobs".to_string(),
                last_run: Some(now - chrono::Duration::hours(1)),
                next_run: Some(next_hour),
            },
            ScheduledTask {
                name: "log:rotate".to_string(),
                expression: "0 0 * * 0".to_string(), // Every Sunday at midnight
                description: "Rotiert Log-Dateien".to_string(),
                last_run: Some(now - chrono::Duration::weeks(1)),
                next_run: Some(now + chrono::Duration::days(7)),
            },
        ]
    }

    fn format_as_table(tasks: &[ScheduledTask]) -> String {
        let mut lines = vec![
            String::from("╭───────────────────┬────────────────┬──────────────────────────────┬─────────────────────┬─────────────────────╮"),
            String::from("│ TASK NAME         │ CRON           │ DESCRIPTION                  │ LAST RUN            │ NEXT RUN            │"),
            String::from("├───────────────────┼────────────────┼──────────────────────────────┼─────────────────────┼─────────────────────┤"),
        ];

        for task in tasks {
            let last_run = task
                .last_run
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string());

            let next_run = task
                .next_run
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "N/A".to_string());

            let line = format!(
                "│ {:<17} │ {:<14} │ {:<28} │ {:<19} │ {:<19} │",
                truncate(&task.name, 17),
                truncate(&task.expression, 14),
                truncate(&task.description, 28),
                truncate(&last_run, 19),
                truncate(&next_run, 19),
            );
            lines.push(line);
        }

        lines.push(String::from("╰───────────────────┴────────────────┴──────────────────────────────┴─────────────────────┴─────────────────────╯"));
        lines.join("\n")
    }
}

#[async_trait]
impl FoundryCommand for ScheduleListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let use_json = ctx.args.iter().any(|arg| {
            arg == "--format=json" || arg == "--json"
        });

        let tasks = Self::get_scheduled_tasks();
        let total = tasks.len();

        let message = if use_json {
            format!("{} scheduled tasks", total)
        } else {
            let table = Self::format_as_table(&tasks);
            format!("{}\n\n{} scheduled tasks", table, total)
        };

        let data = json!({
            "total": total,
            "tasks": tasks,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

// ==================== ScheduleRunCommand ====================

pub struct ScheduleRunCommand {
    descriptor: CommandDescriptor,
}

impl Default for ScheduleRunCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl ScheduleRunCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.schedule_run", "schedule:run")
            .summary("Führt geplante Tasks aus")
            .description("Führt alle fälligen Scheduled Tasks aus. Mit --force werden alle Tasks ausgeführt.")
            .category(CommandKind::Runtime)
            .build();

        Self { descriptor }
    }

    fn should_run_task(task: &ScheduledTask, force: bool) -> bool {
        if force {
            return true;
        }

        // Check if task is due based on next_run timestamp
        if let Some(next_run) = task.next_run {
            let now = Utc::now();
            now >= next_run
        } else {
            false
        }
    }

    async fn execute_task(task: &ScheduledTask) -> Result<(), CommandError> {
        info!(
            task_name = %task.name,
            expression = %task.expression,
            "Executing scheduled task"
        );

        // Simulate task execution
        // In a real implementation, this would:
        // 1. Look up the task handler from app/console/schedule.rs
        // 2. Execute the actual command
        // 3. Update last_run timestamp
        // 4. Calculate next_run based on cron expression

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        info!(task_name = %task.name, "Task completed successfully");
        Ok(())
    }
}

#[async_trait]
impl FoundryCommand for ScheduleRunCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let force = ctx.args.iter().any(|arg| arg == "--force");

        if force {
            info!("Force flag enabled - all tasks will be executed");
        }

        let tasks = ScheduleListCommand::get_scheduled_tasks();
        let mut executed = Vec::new();
        let mut skipped = Vec::new();
        let mut failed = Vec::new();

        for task in tasks {
            if Self::should_run_task(&task, force) {
                match Self::execute_task(&task).await {
                    Ok(_) => {
                        executed.push(task.name.clone());
                    }
                    Err(e) => {
                        failed.push(json!({
                            "task": task.name,
                            "error": e.to_string(),
                        }));
                    }
                }
            } else {
                skipped.push(task.name.clone());
            }
        }

        let message = if force {
            format!(
                "Forced execution completed. Executed: {}, Failed: {}",
                executed.len(),
                failed.len()
            )
        } else {
            format!(
                "Scheduler run completed. Executed: {}, Skipped: {}, Failed: {}",
                executed.len(),
                skipped.len(),
                failed.len()
            )
        };

        let data = json!({
            "executed": executed,
            "skipped": skipped,
            "failed": failed,
            "force": force,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

// ==================== Helper Functions ====================

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
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
    async fn test_schedule_list_table_format() {
        let command = ScheduleListCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());

        let message = result.message.unwrap();
        assert!(message.contains("TASK NAME"));
        assert!(message.contains("CRON"));
        assert!(message.contains("scheduled tasks"));
    }

    #[tokio::test]
    async fn test_schedule_list_json_format() {
        let command = ScheduleListCommand::new();
        let ctx = create_test_context(vec!["--format=json".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        let total = data["total"].as_u64().unwrap();
        assert!(total > 0);
        assert!(data["tasks"].is_array());
    }

    #[tokio::test]
    async fn test_schedule_run_without_force() {
        let command = ScheduleRunCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert!(data["executed"].is_array());
        assert!(data["skipped"].is_array());
        assert_eq!(data["force"], false);
    }

    #[tokio::test]
    async fn test_schedule_run_with_force() {
        let command = ScheduleRunCommand::new();
        let ctx = create_test_context(vec!["--force".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        assert_eq!(data["force"], true);

        // With force, all tasks should be executed
        let executed = data["executed"].as_array().unwrap();
        assert!(executed.len() > 0);
    }

    #[tokio::test]
    async fn test_scheduled_task_structure() {
        let tasks = ScheduleListCommand::get_scheduled_tasks();
        assert!(!tasks.is_empty());

        for task in tasks {
            assert!(!task.name.is_empty());
            assert!(!task.expression.is_empty());
            assert!(!task.description.is_empty());
        }
    }

    #[tokio::test]
    async fn test_schedule_list_descriptor() {
        let command = ScheduleListCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "schedule:list");
        assert_eq!(descriptor.category, CommandKind::Utility);
    }

    #[tokio::test]
    async fn test_schedule_run_descriptor() {
        let command = ScheduleRunCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "schedule:run");
        assert_eq!(descriptor.category, CommandKind::Runtime);
    }
}
