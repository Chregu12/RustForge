//! Asset publishing utilities

use crate::{Package, PackageError, PackageResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Types of assets that can be published
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    /// Configuration files
    Config,
    /// Database migrations
    Migrations,
    /// View templates
    Views,
    /// Public assets (CSS, JS, images)
    Public,
    /// Custom asset type
    Custom(String),
}

impl AssetType {
    /// Get the default publish path for this asset type
    pub fn default_path(&self) -> &str {
        match self {
            AssetType::Config => "config",
            AssetType::Migrations => "migrations",
            AssetType::Views => "resources/views/vendor",
            AssetType::Public => "public/vendor",
            AssetType::Custom(name) => name,
        }
    }
}

/// Publishing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishConfig {
    /// Whether to overwrite existing files
    pub overwrite: bool,

    /// Whether to create backup of existing files
    pub backup: bool,

    /// Backup suffix
    pub backup_suffix: String,

    /// Whether to be verbose
    pub verbose: bool,
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            overwrite: false,
            backup: true,
            backup_suffix: ".bak".to_string(),
            verbose: false,
        }
    }
}

/// Asset publisher
pub struct Publisher {
    package: Package,
    config: PublishConfig,
}

impl Publisher {
    /// Create a new publisher for the given package
    pub fn new(package: Package) -> Self {
        Self {
            package,
            config: PublishConfig::default(),
        }
    }

    /// Create a publisher with custom config
    pub fn with_config(package: Package, config: PublishConfig) -> Self {
        Self { package, config }
    }

    /// Publish assets of the given type to the destination
    pub async fn publish(&self, asset_type: AssetType, dest: impl AsRef<Path>) -> PackageResult<()> {
        let dest = dest.as_ref();

        match asset_type {
            AssetType::Config => self.publish_configs(dest).await,
            AssetType::Migrations => self.publish_migrations(dest).await,
            AssetType::Views => self.publish_views(dest).await,
            AssetType::Public => self.publish_public(dest).await,
            AssetType::Custom(ref name) => self.publish_custom(name, dest).await,
        }
    }

    /// Publish configuration files
    async fn publish_configs(&self, dest: &Path) -> PackageResult<()> {
        for config in &self.package.configs {
            self.copy_file(config, dest).await?;
        }
        Ok(())
    }

    /// Publish migrations
    async fn publish_migrations(&self, dest: &Path) -> PackageResult<()> {
        for migration in &self.package.migrations {
            self.copy_file(migration, dest).await?;
        }
        Ok(())
    }

    /// Publish views
    async fn publish_views(&self, dest: &Path) -> PackageResult<()> {
        for view_dir in &self.package.views {
            self.copy_directory(view_dir, dest).await?;
        }
        Ok(())
    }

    /// Publish public assets
    async fn publish_public(&self, dest: &Path) -> PackageResult<()> {
        let public_dir = self.package.base_path.join("public");
        if public_dir.exists() {
            self.copy_directory(&public_dir, dest).await?;
        }
        Ok(())
    }

    /// Publish custom assets
    async fn publish_custom(&self, name: &str, dest: &Path) -> PackageResult<()> {
        if let Some(asset_path) = self.package.assets.get(name) {
            if asset_path.is_file() {
                self.copy_file(asset_path, dest).await?;
            } else if asset_path.is_dir() {
                self.copy_directory(asset_path, dest).await?;
            }
        }
        Ok(())
    }

    /// Copy a single file
    async fn copy_file(&self, src: &Path, dest_dir: &Path) -> PackageResult<()> {
        if !src.exists() {
            return Ok(());
        }

        std::fs::create_dir_all(dest_dir)?;

        let file_name = src.file_name()
            .ok_or_else(|| PackageError::PublishError("Invalid file name".to_string()))?;
        let dest = dest_dir.join(file_name);

        // Handle existing files
        if dest.exists() && !self.config.overwrite {
            if self.config.verbose {
                println!("Skipping existing file: {}", dest.display());
            }
            return Ok(());
        }

        // Create backup if needed
        if dest.exists() && self.config.backup {
            let backup = dest.with_extension(
                format!("{}{}",
                    dest.extension().and_then(|s| s.to_str()).unwrap_or(""),
                    self.config.backup_suffix
                )
            );
            std::fs::copy(&dest, &backup)?;
            if self.config.verbose {
                println!("Created backup: {}", backup.display());
            }
        }

        // Copy file
        std::fs::copy(src, &dest)?;
        if self.config.verbose {
            println!("Published: {} -> {}", src.display(), dest.display());
        }

        Ok(())
    }

    /// Copy a directory recursively
    fn copy_directory<'a>(&'a self, src: &'a Path, dest: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = PackageResult<()>> + 'a>> {
        Box::pin(async move {
        if !src.exists() {
            return Ok(());
        }

        std::fs::create_dir_all(dest)?;

        let entries = std::fs::read_dir(src)?;
        for entry in entries {
            let entry = entry?;
            let src_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(&file_name);

            if src_path.is_dir() {
                self.copy_directory(&src_path, &dest_path).await?;
            } else {
                if dest_path.exists() && !self.config.overwrite {
                    if self.config.verbose {
                        println!("Skipping existing file: {}", dest_path.display());
                    }
                    continue;
                }

                if dest_path.exists() && self.config.backup {
                    let backup = PathBuf::from(format!("{}{}",
                        dest_path.display(),
                        self.config.backup_suffix
                    ));
                    std::fs::copy(&dest_path, &backup)?;
                }

                std::fs::copy(&src_path, &dest_path)?;
                if self.config.verbose {
                    println!("Published: {} -> {}", src_path.display(), dest_path.display());
                }
            }
        }

        Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_asset_type_default_path() {
        assert_eq!(AssetType::Config.default_path(), "config");
        assert_eq!(AssetType::Migrations.default_path(), "migrations");
        assert_eq!(AssetType::Views.default_path(), "resources/views/vendor");
        assert_eq!(AssetType::Public.default_path(), "public/vendor");
    }

    #[test]
    fn test_publish_config_default() {
        let config = PublishConfig::default();
        assert!(!config.overwrite);
        assert!(config.backup);
        assert_eq!(config.backup_suffix, ".bak");
    }

    #[tokio::test]
    async fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let src_file = temp.path().join("source.txt");
        std::fs::write(&src_file, "test content").unwrap();

        let package = Package::new("test-package")
            .base_path(temp.path())
            .build()
            .unwrap();

        let publisher = Publisher::new(package);
        let dest_dir = temp.path().join("dest");

        publisher.copy_file(&src_file, &dest_dir).await.unwrap();

        let dest_file = dest_dir.join("source.txt");
        assert!(dest_file.exists());
        assert_eq!(std::fs::read_to_string(dest_file).unwrap(), "test content");
    }

    #[tokio::test]
    async fn test_overwrite_protection() {
        let temp = TempDir::new().unwrap();
        let src_file = temp.path().join("source.txt");
        std::fs::write(&src_file, "new content").unwrap();

        let dest_dir = temp.path().join("dest");
        std::fs::create_dir_all(&dest_dir).unwrap();
        let dest_file = dest_dir.join("source.txt");
        std::fs::write(&dest_file, "old content").unwrap();

        let package = Package::new("test-package")
            .base_path(temp.path())
            .build()
            .unwrap();

        let publisher = Publisher::new(package);
        publisher.copy_file(&src_file, &dest_dir).await.unwrap();

        // Should not overwrite
        assert_eq!(std::fs::read_to_string(&dest_file).unwrap(), "old content");
    }
}
