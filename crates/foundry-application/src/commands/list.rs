use crate::CommandRegistry;
use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;

pub struct ListCommand {
    descriptor: CommandDescriptor,
    registry: CommandRegistry,
}

impl ListCommand {
    pub fn new(registry: CommandRegistry) -> Self {
        let descriptor = CommandDescriptor::builder("core.list", "list")
            .summary("Listet alle verfügbaren Foundry-Commands")
            .description("Zeigt eine katalogisierte Übersicht der registrierten Commands und ihrer Metadaten.")
            .category(CommandKind::Core)
            .alias("ls")
            .build();

        Self {
            descriptor,
            registry,
        }
    }
}

#[async_trait]
impl FoundryCommand for ListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let catalog = self.registry.descriptors();
        let total = catalog.len();
        let message = match ctx.format {
            foundry_plugins::ResponseFormat::Human => {
                let mut lines = vec![format!("{} Commands verfügbar:", total)];
                for entry in &catalog {
                    lines.push(format!("  {:<18} {}", entry.name, entry.summary.trim()));
                }
                lines.join("\n")
            }
            foundry_plugins::ResponseFormat::Json => format!("{total} commands available"),
        };

        let data = json!({
            "total": total,
            "commands": catalog,
        });

        let result = CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        };

        Ok(result)
    }
}
