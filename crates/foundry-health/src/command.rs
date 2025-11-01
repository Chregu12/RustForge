//! Health check CLI command

use crate::{HealthCheckConfig, HealthChecker, HealthReport};
use anyhow::Result;
use async_trait::async_trait;
use foundry_plugins::{CommandExecutor, CommandResult, ExecutionContext};

/// Health check command (health:check or doctor)
pub struct HealthCheckCommand;

#[async_trait]
impl CommandExecutor for HealthCheckCommand {
    fn name(&self) -> &'static str {
        "health:check"
    }

    fn description(&self) -> &'static str {
        "Run comprehensive health checks on the application"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
        // Parse arguments for specific check
        let specific_check = ctx.args.first().map(|s| s.as_str());

        // Load environment variables
        let database_url = std::env::var("DATABASE_URL").ok();
        let cache_url = std::env::var("CACHE_URL").ok();

        let config = HealthCheckConfig {
            database_url,
            cache_url,
            ..Default::default()
        };

        let checker = HealthChecker::new(config);

        let results = if let Some(check_name) = specific_check {
            vec![checker.check_one(check_name).await?]
        } else {
            checker.check_all().await?
        };

        let report = HealthReport::new(results);

        // Format output
        let output = match ctx.format {
            foundry_plugins::ResponseFormat::Human => report.format_table(),
            foundry_plugins::ResponseFormat::Json => {
                serde_json::to_string_pretty(&report)?
            }
        };

        if report.all_passed() {
            Ok(CommandResult::success(&output))
        } else {
            Ok(CommandResult::error(&output))
        }
    }
}

/// Doctor command (alias for health:check)
pub struct DoctorCommand;

#[async_trait]
impl CommandExecutor for DoctorCommand {
    fn name(&self) -> &'static str {
        "doctor"
    }

    fn description(&self) -> &'static str {
        "Run comprehensive health checks (alias for health:check)"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
        HealthCheckCommand.execute(ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::{ExecutionOptions, ResponseFormat};

    #[tokio::test]
    async fn test_health_check_command() {
        let cmd = HealthCheckCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_health_check_specific() {
        let cmd = HealthCheckCommand;
        let ctx = ExecutionContext {
            args: vec!["rust".to_string()],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_doctor_command() {
        let cmd = DoctorCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.message.is_some());
    }
}
