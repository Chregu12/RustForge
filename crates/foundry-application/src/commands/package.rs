//! Package Management Commands
//!
//! Composer-ähnliche Commands für Rust Package Management.

use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_infra::PackageManager;
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;

/// Package Install Command
pub struct PackageInstallCommand {
    descriptor: CommandDescriptor,
}

impl PackageInstallCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.install", "package:install")
                .summary("Installiert ein Package")
                .description("Installiert ein Rust Package von crates.io")
                .category(CommandKind::System)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageInstallCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let package_name = ctx.args.get("package")
            .ok_or_else(|| CommandError::MissingArgument("package".to_string()))?
            .as_str()
            .ok_or_else(|| CommandError::InvalidArgument("package".to_string()))?;

        let version = ctx.args.get("version")
            .and_then(|v| v.as_str());

        let pm = PackageManager::new(".");
        pm.install(package_name, version)
            .await
            .map_err(|e| CommandError::Execution(e.to_string()))?;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(format!("Package {} installed successfully", package_name)),
            data: Some(json!({
                "package": package_name,
                "version": version.unwrap_or("latest"),
            })),
            error: None,
        })
    }
}

impl Default for PackageInstallCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Package Remove Command
pub struct PackageRemoveCommand {
    descriptor: CommandDescriptor,
}

impl PackageRemoveCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.remove", "package:remove")
                .summary("Entfernt ein Package")
                .description("Entfernt ein installiertes Package")
                .category(CommandKind::System)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageRemoveCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let package_name = ctx.args.get("package")
            .ok_or_else(|| CommandError::MissingArgument("package".to_string()))?
            .as_str()
            .ok_or_else(|| CommandError::InvalidArgument("package".to_string()))?;

        let pm = PackageManager::new(".");
        pm.remove(package_name)
            .await
            .map_err(|e| CommandError::Execution(e.to_string()))?;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(format!("Package {} removed successfully", package_name)),
            data: Some(json!({"package": package_name})),
            error: None,
        })
    }
}

impl Default for PackageRemoveCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Package Update Command
pub struct PackageUpdateCommand {
    descriptor: CommandDescriptor,
}

impl PackageUpdateCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.update", "package:update")
                .summary("Aktualisiert Packages")
                .description("Aktualisiert alle oder ein spezifisches Package")
                .category(CommandKind::System)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageUpdateCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let package_name = ctx.args.get("package").and_then(|v| v.as_str());

        let pm = PackageManager::new(".");

        if let Some(package) = package_name {
            pm.update_package(package)
                .await
                .map_err(|e| CommandError::Execution(e.to_string()))?;

            Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some(format!("Package {} updated successfully", package)),
                data: Some(json!({"package": package})),
                error: None,
            })
        } else {
            pm.update()
                .await
                .map_err(|e| CommandError::Execution(e.to_string()))?;

            Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some("All packages updated successfully".to_string()),
                data: None,
                error: None,
            })
        }
    }
}

impl Default for PackageUpdateCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Package List Command
pub struct PackageListCommand {
    descriptor: CommandDescriptor,
}

impl PackageListCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.list", "package:list")
                .summary("Listet installierte Packages")
                .description("Zeigt alle installierten Packages an")
                .category(CommandKind::Monitoring)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageListCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let pm = PackageManager::new(".");
        let packages = pm.list()
            .await
            .map_err(|e| CommandError::Execution(e.to_string()))?;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(format!("Found {} installed packages", packages.len())),
            data: Some(json!({"packages": packages})),
            error: None,
        })
    }
}

impl Default for PackageListCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Package Search Command
pub struct PackageSearchCommand {
    descriptor: CommandDescriptor,
}

impl PackageSearchCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.search", "package:search")
                .summary("Sucht nach Packages")
                .description("Sucht auf crates.io nach Packages")
                .category(CommandKind::System)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageSearchCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let query = ctx.args.get("query")
            .ok_or_else(|| CommandError::MissingArgument("query".to_string()))?
            .as_str()
            .ok_or_else(|| CommandError::InvalidArgument("query".to_string()))?;

        let pm = PackageManager::new(".");
        let results = pm.search(query)
            .await
            .map_err(|e| CommandError::Execution(e.to_string()))?;

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(format!("Found {} packages", results.len())),
            data: Some(json!({"results": results})),
            error: None,
        })
    }
}

impl Default for PackageSearchCommand {
    fn default() -> Self {
        Self::new()
    }
}

/// Package Outdated Command
pub struct PackageOutdatedCommand {
    descriptor: CommandDescriptor,
}

impl PackageOutdatedCommand {
    pub fn new() -> Self {
        Self {
            descriptor: CommandDescriptor::builder("package.outdated", "package:outdated")
                .summary("Zeigt veraltete Packages")
                .description("Listet alle veralteten Packages auf")
                .category(CommandKind::Monitoring)
                .build(),
        }
    }
}

#[async_trait]
impl FoundryCommand for PackageOutdatedCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, _ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let pm = PackageManager::new(".");
        let outdated = pm.outdated()
            .await
            .map_err(|e| CommandError::Execution(e.to_string()))?;

        if outdated.is_empty() {
            Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some("All packages are up to date".to_string()),
                data: None,
                error: None,
            })
        } else {
            Ok(CommandResult {
                status: CommandStatus::Success,
                message: Some(format!("Found {} outdated packages", outdated.len())),
                data: Some(json!({"outdated": outdated})),
                error: None,
            })
        }
    }
}

impl Default for PackageOutdatedCommand {
    fn default() -> Self {
        Self::new()
    }
}
