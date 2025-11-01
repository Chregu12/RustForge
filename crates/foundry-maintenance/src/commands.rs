//! CLI commands for maintenance mode

use crate::{MaintenanceConfig, MaintenanceMode};
use anyhow::Result;
use async_trait::async_trait;
use foundry_plugins::{CommandExecutor, CommandResult, ExecutionContext};
use serde_json::json;

/// Command to enable maintenance mode (app:down)
pub struct AppDownCommand;

#[async_trait]
impl CommandExecutor for AppDownCommand {
    fn name(&self) -> &'static str {
        "app:down"
    }

    fn description(&self) -> &'static str {
        "Put the application into maintenance mode"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
        // Parse arguments
        let mut message = None;
        let mut secret = None;
        let mut retry_after = None;

        let mut i = 0;
        while i < ctx.args.len() {
            match ctx.args[i].as_str() {
                "--message" => {
                    if i + 1 < ctx.args.len() {
                        message = Some(ctx.args[i + 1].clone());
                        i += 2;
                    } else {
                        return Ok(CommandResult::error("--message requires a value"));
                    }
                }
                "--secret" => {
                    if i + 1 < ctx.args.len() {
                        secret = Some(ctx.args[i + 1].clone());
                        i += 2;
                    } else {
                        return Ok(CommandResult::error("--secret requires a value"));
                    }
                }
                "--retry" => {
                    if i + 1 < ctx.args.len() {
                        retry_after = Some(
                            ctx.args[i + 1]
                                .parse::<u64>()
                                .map_err(|_| anyhow::anyhow!("Invalid retry value"))?,
                        );
                        i += 2;
                    } else {
                        return Ok(CommandResult::error("--retry requires a value"));
                    }
                }
                _ => {
                    i += 1;
                }
            }
        }

        let config = MaintenanceConfig {
            file_path: ".maintenance".into(),
            message: message.or_else(|| {
                Some("Application is down for maintenance. Please check back soon.".to_string())
            }),
            secret,
        };

        if ctx.options.dry_run {
            return Ok(CommandResult::success(
                "Would enable maintenance mode (dry run)",
            )
            .with_data(json!({
                "message": config.message,
                "secret_set": config.secret.is_some(),
                "retry_after": retry_after,
            })));
        }

        let mode = MaintenanceMode::new(config);

        if let Some(retry) = retry_after {
            mode.enable_with_retry(retry)?;
        } else {
            mode.enable()?;
        }

        Ok(CommandResult::success("Application is now in maintenance mode")
            .with_data(json!({
                "file": ".maintenance",
                "secret_set": mode.config().secret.is_some(),
            })))
    }
}

/// Command to disable maintenance mode (app:up)
pub struct AppUpCommand;

#[async_trait]
impl CommandExecutor for AppUpCommand {
    fn name(&self) -> &'static str {
        "app:up"
    }

    fn description(&self) -> &'static str {
        "Bring the application out of maintenance mode"
    }

    async fn execute(&self, ctx: &ExecutionContext) -> Result<CommandResult> {
        let config = MaintenanceConfig {
            file_path: ".maintenance".into(),
            message: None,
            secret: None,
        };

        if ctx.options.dry_run {
            return Ok(CommandResult::success(
                "Would disable maintenance mode (dry run)",
            ));
        }

        let mode = MaintenanceMode::new(config);

        if !mode.is_active() {
            return Ok(CommandResult::success("Application is not in maintenance mode"));
        }

        mode.disable()?;

        Ok(CommandResult::success("Application is now live"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::{ExecutionOptions, ResponseFormat};
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_app_down_command() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = AppDownCommand;
        let ctx = ExecutionContext {
            args: vec!["--message".to_string(), "Test maintenance".to_string()],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.is_success());

        assert!(temp_dir.path().join(".maintenance").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_app_down_with_secret() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = AppDownCommand;
        let ctx = ExecutionContext {
            args: vec!["--secret".to_string(), "mysecret123".to_string()],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.is_success());

        let content = fs::read_to_string(".maintenance").unwrap();
        assert!(content.contains("mysecret123"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_app_up_command() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        // First enable
        fs::write(".maintenance", "{}").unwrap();

        let cmd = AppUpCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.is_success());

        assert!(!temp_dir.path().join(".maintenance").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_dry_run() {
        let cmd = AppDownCommand;
        let ctx = ExecutionContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: true,
                force: false,
            },
        };

        let result = cmd.execute(&ctx).await.unwrap();
        assert!(result.is_success());
        assert!(result.message.unwrap().contains("dry run"));
    }
}
