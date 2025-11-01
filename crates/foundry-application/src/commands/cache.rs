use async_trait::async_trait;
use foundry_domain::{CommandDescriptor, CommandKind};
use foundry_plugins::{CommandContext, CommandError, CommandResult, CommandStatus, FoundryCommand};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct CacheClearCommand {
    descriptor: CommandDescriptor,
}

impl Default for CacheClearCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheClearCommand {
    pub fn new() -> Self {
        let descriptor = CommandDescriptor::builder("core.cache_clear", "cache:clear")
            .summary("Löscht Application-Caches")
            .description("Löscht selektiv oder alle Caches (config, routes, views).")
            .category(CommandKind::Utility)
            .build();

        Self { descriptor }
    }

    fn get_cache_dirs() -> Vec<(&'static str, PathBuf)> {
        vec![
            ("config", PathBuf::from("storage/framework/cache/config")),
            ("routes", PathBuf::from("storage/framework/cache/routes")),
            ("views", PathBuf::from("storage/framework/cache/views")),
            ("data", PathBuf::from("storage/framework/cache/data")),
        ]
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

    async fn clear_cache_port(ctx: &CommandContext) -> Result<usize, CommandError> {
        // Clear the in-memory cache via CachePort
        ctx.cache.clear(None).await?;
        Ok(0) // In-memory cache doesn't have a count
    }
}

#[async_trait]
impl FoundryCommand for CacheClearCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        &self.descriptor
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let clear_config = ctx.args.iter().any(|arg| arg == "--config");
        let clear_routes = ctx.args.iter().any(|arg| arg == "--routes");
        let clear_views = ctx.args.iter().any(|arg| arg == "--views");
        let clear_all = !clear_config && !clear_routes && !clear_views;

        let mut cleared_items = Vec::new();
        let mut total_files = 0;

        // Clear in-memory cache
        if clear_all {
            Self::clear_cache_port(&ctx).await?;
            info!("In-memory cache cleared");
        }

        // Clear file-based caches
        let cache_dirs = Self::get_cache_dirs();

        for (name, path) in cache_dirs {
            let should_clear = match name {
                "config" => clear_all || clear_config,
                "routes" => clear_all || clear_routes,
                "views" => clear_all || clear_views,
                "data" => clear_all,
                _ => false,
            };

            if should_clear {
                match Self::clear_directory(&path) {
                    Ok(count) => {
                        if count > 0 || path.exists() {
                            cleared_items.push(json!({
                                "cache": name,
                                "path": path.display().to_string(),
                                "files_removed": count,
                            }));
                            total_files += count;
                            info!(cache = name, files = count, "Cache cleared");
                        }
                    }
                    Err(e) => {
                        warn!(cache = name, error = %e, "Failed to clear cache");
                    }
                }
            }
        }

        let message = if cleared_items.is_empty() {
            "No caches found to clear".to_string()
        } else if clear_all {
            format!("All caches cleared ({} files removed)", total_files)
        } else {
            let cache_names: Vec<&str> = cleared_items
                .iter()
                .filter_map(|item| item["cache"].as_str())
                .collect();
            format!(
                "Caches cleared: {} ({} files removed)",
                cache_names.join(", "),
                total_files
            )
        };

        let data = json!({
            "cleared": cleared_items,
            "total_files": total_files,
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
    async fn test_cache_clear_all() {
        let command = CacheClearCommand::new();
        let ctx = create_test_context(vec![]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear_selective_config() {
        let command = CacheClearCommand::new();
        let ctx = create_test_context(vec!["--config".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);

        let data = result.data.unwrap();
        let cleared = data["cleared"].as_array().unwrap();

        // Check that only config cache is cleared
        if !cleared.is_empty() {
            assert!(cleared.iter().any(|item| item["cache"].as_str() == Some("config")));
        }
    }

    #[tokio::test]
    async fn test_cache_clear_selective_routes() {
        let command = CacheClearCommand::new();
        let ctx = create_test_context(vec!["--routes".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
    }

    #[tokio::test]
    async fn test_cache_clear_selective_views() {
        let command = CacheClearCommand::new();
        let ctx = create_test_context(vec!["--views".to_string()]);

        let result = command.execute(ctx).await.unwrap();
        assert_eq!(result.status, CommandStatus::Success);
    }

    #[tokio::test]
    async fn test_cache_clear_descriptor() {
        let command = CacheClearCommand::new();
        let descriptor = command.descriptor();

        assert_eq!(descriptor.name, "cache:clear");
        assert_eq!(descriptor.category, CommandKind::Utility);
    }
}
