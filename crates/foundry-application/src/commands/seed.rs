use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{
    CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand, ResponseFormat,
};
use serde_json::json;

pub struct SeedCommand {
    descriptor: CommandDescriptor,
}

impl SeedCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("database.seed", "seed")
                .summary("Führt deterministische Seed-Skripte aus")
                .description("Plant oder führt Seed-Skripte über SeaORM aus.")
                .category(CommandKind::Database)
                .alias("db:seed")
                .build(),
        }
    }
}

impl Default for SeedCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FoundryCommand for SeedCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let format = ctx.format.clone();
        let args_snapshot = ctx.args.clone();
        let options = ctx.options;

        let run = ctx.seeds.run(&ctx.config, options.dry_run).await?;
        let message = match format {
            ResponseFormat::Human => {
                if options.dry_run {
                    format!("seed → {} Seed(s) geplant (dry-run).", run.pending.len())
                } else {
                    format!("seed → {} Seed(s) ausgeführt.", run.executed.len())
                }
            }
            ResponseFormat::Json => {
                if options.dry_run {
                    "planned seed".to_string()
                } else {
                    "executed seed".to_string()
                }
            }
        };

        let data = json!({
            "run": run,
            "input": {
                "args": args_snapshot,
            },
            "dry_run": options.dry_run,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}
