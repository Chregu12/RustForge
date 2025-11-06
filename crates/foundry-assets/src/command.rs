//! Asset publishing CLI command

use crate::publisher::{AssetPublisher, PublishConfig};
use async_trait::async_trait;
use foundry_plugins::{FoundryCommand, CommandResult, CommandContext, CommandError};
use std::path::PathBuf;

/// Asset publishing command
pub struct AssetPublishCommand;

#[async_trait]
impl FoundryCommand for AssetPublishCommand {
    fn descriptor(&self) -> &foundry_domain::CommandDescriptor {
        use std::sync::OnceLock;
        static DESCRIPTOR: OnceLock<foundry_domain::CommandDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            foundry_domain::CommandDescriptor::builder("asset:publish", "publish")
                .description("Publish static assets to public directory with cache busting")
                .build()
        })
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        // Parse arguments
        let mut source_dir = PathBuf::from("assets");
        let mut target_dir = PathBuf::from("public");
        let mut versioning = true;

        let mut i = 0;
        while i < ctx.args.len() {
            match ctx.args[i].as_str() {
                "--source" => {
                    if i + 1 < ctx.args.len() {
                        source_dir = PathBuf::from(&ctx.args[i + 1]);
                        i += 2;
                    } else {
                        return Err(CommandError::Message("--source requires a value".to_string()));
                    }
                }
                "--target" => {
                    if i + 1 < ctx.args.len() {
                        target_dir = PathBuf::from(&ctx.args[i + 1]);
                        i += 2;
                    } else {
                        return Err(CommandError::Message("--target requires a value".to_string()));
                    }
                }
                "--no-versioning" => {
                    versioning = false;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        if !source_dir.exists() {
            return Err(CommandError::Message(format!(
                "Source directory '{}' does not exist",
                source_dir.display()
            )));
        }

        if ctx.options.dry_run {
            return Ok(CommandResult::success(&format!(
                "Would publish assets from {} to {} (dry run)",
                source_dir.display(),
                target_dir.display()
            )));
        }

        let config = PublishConfig {
            source_dir: source_dir.clone(),
            target_dir: target_dir.clone(),
            versioning,
            ..Default::default()
        };

        let publisher = AssetPublisher::new(config);
        let result = publisher.publish()
            .map_err(|e| CommandError::Other(e))?;

        // Save manifest
        let manifest_path = target_dir.join("asset-manifest.json");
        result.manifest.save(&manifest_path)
            .map_err(|e| CommandError::Other(e))?;

        let message = format!(
            "Published {} files ({} bytes) to {}\nManifest saved to {}",
            result.files_published,
            result.bytes_copied,
            target_dir.display(),
            manifest_path.display()
        );

        Ok(CommandResult::success(&message).with_data(serde_json::json!({
            "files_published": result.files_published,
            "bytes_copied": result.bytes_copied,
            "manifest_path": manifest_path.to_string_lossy(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::{ExecutionOptions, ResponseFormat};
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_asset_publish_command() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(assets_dir.join("test.js"), "console.log('test');").unwrap();

        let cmd = AssetPublishCommand;
        let ctx = CommandContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: false,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.is_success());

        assert!(temp_dir.path().join("public").exists());
        assert!(temp_dir.path().join("public/asset-manifest.json").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let assets_dir = temp_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();

        let cmd = AssetPublishCommand;
        let ctx = CommandContext {
            args: vec![],
            format: ResponseFormat::Human,
            options: ExecutionOptions {
                dry_run: true,
                force: false,
            },
        };

        let result = cmd.execute(ctx).await.unwrap();
        assert!(result.is_success());
        assert!(result.message.unwrap().contains("dry run"));

        std::env::set_current_dir(original_dir).unwrap();
    }
}
