use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, FoundryCommand};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

const MAINTENANCE_FILE: &str = ".foundry/down";

pub struct DownCommand {
    descriptor: CommandDescriptor,
}

pub struct UpCommand {
    descriptor: CommandDescriptor,
}

impl Default for DownCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl DownCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("framework.down", "down")
                .summary("Versetzt die Anwendung in den Wartungsmodus")
                .description("Erstellt eine \"down\"-Datei, um den Wartungsmodus zu aktivieren.")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

impl Default for UpCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl UpCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("framework.up", "up")
                .summary("Beendet den Wartungsmodus der Anwendung")
                .description("Löscht die \"down\"-Datei, um den Wartungsmodus zu beenden.")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for DownCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let path = PathBuf::from(MAINTENANCE_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| CommandError::Other(e.into()))?;
        }

        let content = json!({
            "message": "Application is in maintenance mode.",
            "time": chrono::Utc::now().to_rfc3339(),
        });

        fs::write(&path, serde_json::to_string_pretty(&content).unwrap())
            .map_err(|e| CommandError::Other(e.into()))?;

        Ok(CommandResult::success("Anwendung ist jetzt im Wartungsmodus."))
    }
}

#[async_trait]
impl FoundryCommand for UpCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let path = PathBuf::from(MAINTENANCE_FILE);
        if path.exists() {
            fs::remove_file(path).map_err(|e| CommandError::Other(e.into()))?;
            Ok(CommandResult::success("Anwendung ist jetzt wieder online."))
        } else {
            Ok(CommandResult::success("Anwendung war nicht im Wartungsmodus."))
        }
    }
}
