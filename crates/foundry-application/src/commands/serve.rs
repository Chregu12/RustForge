use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;

pub struct ServeCommand {
    descriptor: CommandDescriptor,
}

impl ServeCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("runtime.serve", "serve")
                .summary("Startet den Foundry HTTP/MCP Server (über CLI handled)")
                .description(
                    "Dieses Kommando wird direkt über die CLI implementiert. Verwende `foundry serve`",
                )
                .category(CommandKind::Runtime)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for ServeCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(
                "Nutze `foundry serve [--addr <ADDR>] [--mcp-stdio]`, um den Server zu starten."
                    .to_string(),
            ),
            data: Some(json!({
                "command": "foundry serve",
                "options": ["--addr <ADDR>", "--mcp-stdio"],
            })),
            error: None,
        })
    }
}
