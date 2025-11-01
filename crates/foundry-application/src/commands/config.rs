use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct ConfigClearCommand {
    descriptor: CommandDescriptor,
}

impl Default for ConfigClearCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigClearCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.config_clear", "config:clear")
            .summary("Löscht Config-Caches")
            .description("Löscht nur Config-relevante Caches aus dem storage/framework/cache/config Verzeichnis.")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn get_config_cache_path() -> PathBuf {
        PathBuf::from("storage/framework/cache/config")
    }

    fn clear_directory(path: &Path) -> Result<usize, std::io::Error> {
        if !path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();

                if entry_path.is_file() {
                    fs::remove_file(&entry_path)?;
                    count += 1;
                } else if entry_path.is_dir() {
                    let sub_count = Self::clear_directory_recursive(&entry_path)?;
                    count += sub_count;
                }
            }
        }
        Ok(count)
    }

    fn clear_directory_recursive(path: &Path) -> Result<usize, std::io::Error> {
        let mut count = 0;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let entry_path = entry.path();

                if entry_path.is_file() {
                    fs::remove_file(&entry_path)?;
                    count += 1;
                } else if entry_path.is_dir() {
                    count += Self::clear_directory_recursive(&entry_path)?;
                    fs::remove_dir(&entry_path)?;
                }
            }
        }
        Ok(count)
    }

    async fn clear_cache_port(ctx: &CommandContext) -> Result<(), CommandError> {
        // Clear config-related entries from in-memory cache
        ctx.cache.clear(Some("config")).await?;
        Ok(())
    }
}

#[async_trait]
impl FoundryCommand for ConfigClearCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let mut total_files = 0;

        // Clear in-memory config cache
        Self::clear_cache_port(&ctx).await?;
        info!("In-memory config cache cleared");

        // Clear file-based config cache
        let config_path = Self::get_config_cache_path();

        match Self::clear_directory(&config_path) {
            Ok(count) => {
                total_files = count;
                if count > 0 {
                    info!(files = count, "Config cache files cleared");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to clear config cache directory");
            }
        }

        let message = if total_files == 0 {
            "Config cache: no files to clear".to_string()
        } else {
            format!("Config cache cleared ({} files removed)", total_files)
        };

        let data = json!({
            "cache": "config",
            "path": config_path.display().to_string(),
            "files_removed": total_files,
        });

        Ok(CommandResult {
            status: CommandStatus::Success,
            message: Some(message),
            data: Some(data),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::ResponseFormat;
    use serde_json::Value;

    fn create_test_context(args: Vec<String>) -> CommandContext {
        CommandContext {
            args,
            format: ResponseFormat::Human,
            metadata: Value::Null,
            config: Value::Null,
            options: Default::default(),
            artifacts: std::sync::Arc::new(foundry_infra::LocalArtifactPort::default()),
            migrations: std::sync::Arc::new(foundry_infra::SeaOrmMigrationService::default()),
            seeds: std::sync::Arc::new(foundry_infra::SeaOrmSeedService::default()),
            validation: std::sync::Arc::new(foundry_infra::SimpleValidationService::default()),
            storage: std::sync::Arc::new(foundry_infra::FileStorageAdapter::new(
                std::sync::Arc::new(foundry_storage::manager::StorageManager::new(
                    foundry_storage::config::StorageConfig::from_env()
                ).unwrap())
            )),
            cache: std::sync::Arc::new(foundry_infra::InMemoryCacheStore::default()),
            queue: std::sync::Arc::new(foundry_infra::InMemoryQueue::default()),
            events: std::sync::Arc::new(foundry_infra::InMemoryEventBus::default()),
        }
    }

    #[tokio::test]
    async fn test_config_clear() {
        let command = ConfigClearCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());

        let message = result.message.unwrap();
        assert!(message.contains("Config cache"));
    }

    #[tokio::test]
    async fn test_config_clear_descriptor() {
        let command = ConfigClearCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "config:clear");
        assert_eq!(descriptor.category, CommandKind::Utility);
        assert!(descriptor.summary.contains("Config"));
    }

    #[tokio::test]
    async fn test_config_clear_data_structure() {
        let command = ConfigClearCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        let data = result.data.unwrap();

        assert!(data["cache"].is_string());
        assert_eq!(data["cache"], "config");
        assert!(data["path"].is_string());
        assert!(data["files_removed"].is_number());
    }
}
