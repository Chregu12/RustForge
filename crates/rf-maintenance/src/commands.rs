//! CLI commands for maintenance mode

use crate::{MaintenanceConfig, MaintenanceMode};
use async_trait::async_trait;
use rf_plugins::{CommandContext, CommandResult, FoundryCommand};
use serde_json::json;

/// Command to enable maintenance mode (app:down)
pub struct AppDownCommand;

#[async_trait]
impl FoundryCommand for AppDownCommand {
    fn descriptor(&self) -> &rf_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<rf_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            rf_domain::CommandDescriptor::builder("app:down", "down")
                .description("Put the application into maintenance mode")
                .build()
        })
    }

    async fn execute(
        &self,
        ctx: CommandContext,
    ) -> Result<CommandResult, rf_plugins::CommandError> {
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
                        return Err(rf_plugins::CommandError::Message(
                            "--message requires a value".to_string(),
                        ));
                    }
                }
                "--secret" => {
                    if i + 1 < args.len() {
                        secret = Some(args[i + 1].clone());
                        i += 2;
                    } else {
                        return Err(rf_plugins::CommandError::Message(
                            "--secret requires a value".to_string(),
                        ));
                    }
                }
                "--retry" => {
                    if i + 1 < args.len() {
                        retry_after = Some(args[i + 1].parse::<u64>().map_err(|_| {
                            rf_plugins::CommandError::Message(
                                "Invalid retry value".to_string(),
                            )
                        })?);
                        i += 2;
                    } else {
                        return Err(rf_plugins::CommandError::Message(
                            "--retry requires a value".to_string(),
                        ));
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
            return Ok(
                CommandResult::success("Would enable maintenance mode (dry run)").with_data(
                    json!({
                        "message": config.message,
                        "secret_set": config.secret.is_some(),
                        "retry_after": retry_after,
                    }),
                ),
            );
        }

        let mode = MaintenanceMode::new(config);

        if let Some(retry) = retry_after {
            mode.enable_with_retry(retry)
                .map_err(|e| rf_plugins::CommandError::Other(e))?;
        } else {
            mode.enable()
                .map_err(|e| rf_plugins::CommandError::Other(e))?;
        }

        Ok(
            CommandResult::success("Application is now in maintenance mode").with_data(json!({
                "file": ".maintenance",
                "secret_set": mode.config().secret.is_some(),
            })),
        )
    }
}

/// Command to disable maintenance mode (app:up)
pub struct AppUpCommand;

#[async_trait]
impl FoundryCommand for AppUpCommand {
    fn descriptor(&self) -> &rf_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<rf_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            rf_domain::CommandDescriptor::builder("app:up", "up")
                .description("Bring the application out of maintenance mode")
                .build()
        })
    }

    async fn execute(
        &self,
        ctx: CommandContext,
    ) -> Result<CommandResult, rf_plugins::CommandError> {
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
            return Ok(CommandResult::success(
                "Application is not in maintenance mode",
            ));
        }

        mode.disable()
            .map_err(|e| rf_plugins::CommandError::Other(e))?;

        Ok(CommandResult::success("Application is now live"))
    }
}

// TODO: Update tests to use new CommandContext API
// #[cfg(test)]
// mod tests { ... }
