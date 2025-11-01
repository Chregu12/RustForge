use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;

pub struct EnvCommand {
    descriptor: CommandDescriptor,
}

impl Default for EnvCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("framework.env", "env")
                .summary("Zeigt die Umgebungsvariablen der Anwendung an")
                .description("Zeigt alle geladenen Umgebungsvariablen aus der .env-Datei an.")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for EnvCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let mut variables = Vec::new();
        if let Some(map) = ctx.config.as_object() {
            for (key, value) in map {
                // Handle non-string JSON values gracefully
                let value_str = if let Some(s) = value.as_str() {
                    s.to_string()
                } else {
                    value.to_string()
                };
                variables.push(format!("{key}={value_str}"));
            }
        }
        variables.sort();

        let message = variables.join("\n");

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(json!({ "variables": variables })),
            error: None,
        })
    }
}
