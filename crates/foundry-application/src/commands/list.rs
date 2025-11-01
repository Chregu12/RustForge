use crate::CommandRegistry;
use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use foundry_console::{Table, TableRow, TableCell, BorderStyle, Colorize};
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
                let mut table = Table::new()
                    .with_headers(vec![
                        "Command".to_string(),
                        "Category".to_string(),
                        "Summary".to_string(),
                    ])
                    .border_style(BorderStyle::Rounded)
                    .title(format!("Available Commands ({})", total).green().bold());

                for entry in &catalog {
                    let category_str = format!("{:?}", entry.category);
                    let category_colored = match entry.category {
                        CommandKind::Core => category_str.cyan(),
                        CommandKind::Generator => category_str.green(),
                        CommandKind::Migration => category_str.yellow(),
                        CommandKind::Cache => category_str.magenta(),
                        CommandKind::Queue => category_str.blue(),
                        _ => category_str,
                    };

                    table.add_row(TableRow::new(vec![
                        TableCell::new(entry.name.clone().bold()),
                        TableCell::new(category_colored),
                        TableCell::new(entry.summary.trim()),
                    ]));
                }

                table.render()
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
