//! CLI commands for maintenance mode

use crate::{MaintenanceConfig, MaintenanceMode};
use async_trait::async_trait;
use foundry_plugins::{FoundryCommand, CommandResult, CommandContext};
use serde_json::json;

/// Command to enable maintenance mode (app:down)
pub struct AppDownCommand;

#[async_trait]
impl FoundryCommand for AppDownCommand {
    fn descriptor(&self) -> &foundry_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<foundry_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            foundry_domain::CommandDescriptor::builder("app:down", "down")
                .description("Put the application into maintenance mode")
                .build()
        })
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, foundry_plugins::CommandError> {
        let args = ctx.args;
        let opts = ctx.options;
        // Parse arguments
        let mut message = None;
        let mut secret = None;
        let mut retry_after = None;

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--message" => {
                    if i + 1 < args.len() {
                        message = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(foundry_plugins::CommandError::Message("--message requires a value".to_string()));
                    }
                }
                "--secret" => {
                    if i + 1 < args.len() {
                        secret = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(foundry_plugins::CommandError::Message("--secret requires a value".to_string()));
                    }
                }
                "--retry" => {
                    if i + 1 < args.len() {
                        retry_after = Some(
                            args[i + 1]
                                .parse::<u64>()
                                .map_err(|_| foundry_plugins::CommandError::Message("Invalid retry value".to_string()))?,
                        );
                        i += 2;
                    } else {
                        return Err(foundry_plugins::CommandError::Message("--retry requires a value".to_string()));
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

        if opts.dry_run {
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
            mode.enable_with_retry(retry)
                .map_err(|e| foundry_plugins::CommandError::Other(e))?;
        } else {
            mode.enable()
                .map_err(|e| foundry_plugins::CommandError::Other(e))?;
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
impl FoundryCommand for AppUpCommand {
    fn descriptor(&self) -> &foundry_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<foundry_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            foundry_domain::CommandDescriptor::builder("app:up", "up")
                .description("Bring the application out of maintenance mode")
                .build()
        })
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, foundry_plugins::CommandError> {
        let opts = ctx.options;
        let config = MaintenanceConfig {
            file_path: ".maintenance".into(),
            message: None,
            secret: None,
        };

        if opts.dry_run {
            return Ok(CommandResult::success(
                "Would disable maintenance mode (dry run)",
            ));
        }

        let mode = MaintenanceMode::new(config);

        if !mode.is_active() {
            return Ok(CommandResult::success("Application is not in maintenance mode"));
        }

        mode.disable()
            .map_err(|e| foundry_plugins::CommandError::Other(e))?;

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
        let ctx = CommandContext {
            args: vec!["--message".to_string(), "Test maintenance".to_string()],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
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
        let ctx = CommandContext {
            args: vec!["--secret".to_string(), "mysecret123".to_string()],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
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
        let ctx = CommandContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.is_success());

        assert!(!temp_dir.path().join(".maintenance").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_dry_run() {
        let cmd = AppDownCommand;
        let ctx = CommandContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: true,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.is_success());
        assert!(result.message.unwrap().contains("dry run"));
    }
}
