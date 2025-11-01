use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde::Deserialize;
use serde_json::json;
use std::process::Command;
use std::fs;

#[derive(Deserialize)]
#[allow(dead_code)]
struct WorkspacePackage {
    name: Option<String>,
    version: Option<String>,
    authors: Option<Vec<String>>,
    edition: Option<String>,
}

#[derive(Deserialize)]
struct Workspace {
    package: Option<WorkspacePackage>,
}

#[derive(Deserialize)]
struct CargoToml {
    workspace: Option<Workspace>,
}

pub struct AboutCommand {
    descriptor: CommandDescriptor,
}

impl Default for AboutCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl AboutCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("framework.about", "about")
                .summary("Zeigt Detailinformationen über die Anwendungsumgebung")
                .description("Sammelt und zeigt Details zur Anwendung, wie Version, Rust-Compiler und Datenbank-Status.")
                .category(CommandKind::Utility)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for AboutCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let mut details = Vec::new();

        // Get Rust and Cargo version
        if let Ok(output) = Command::new("rustc").arg("-V").output() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                details.push(("Rust Version", version.trim().to_string()));
            }
        }

        // Get app info from Cargo.toml
        if let Ok(toml_content) = fs::read_to_string("Cargo.toml") {
            if let Ok(cargo_toml) = toml::from_str::<CargoToml>(&toml_content) {
                if let Some(workspace) = cargo_toml.workspace {
                    if let Some(package) = workspace.package {
                        if let Some(version) = package.version {
                            details.push(("App Version", version));
                        }
                        if let Some(edition) = package.edition {
                            details.push(("Rust Edition", edition));
                        }
                    }
                }
            }
        }

        // Get environment info from context
        if let Some(db_driver) = ctx.config.get("DB_CONNECTION").and_then(|v| v.as_str()) {
            details.push(("Database Driver", db_driver.to_string()));
        }

        let message: String = details.iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(json!(details)),
            error: None,
        })
    }
}
