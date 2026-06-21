//! Package definition and builder

use crate::{AssetType, Publisher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PackageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Package name is required")]
    NameRequired,

    #[error("Invalid package name: {0}")]
    InvalidName(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Publishing error: {0}")]
    PublishError(String),
}

pub type PackageResult<T> = Result<T, PackageError>;

/// Package metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package name (e.g., "my-awesome-package")
    pub name: String,

    /// Package description
    pub description: Option<String>,

    /// Package author
    pub author: Option<String>,

    /// Package version
    pub version: String,

    /// Base path for the package
    pub base_path: PathBuf,

    /// Config file paths
    pub configs: Vec<PathBuf>,

    /// Migration file paths
    pub migrations: Vec<PathBuf>,

    /// View directories
    pub views: Vec<PathBuf>,

    /// Custom assets
    pub assets: HashMap<String, PathBuf>,

    /// Tags for asset publishing
    pub tags: HashMap<String, Vec<AssetType>>,
}

impl Package {
    /// Create a new package with the given name
    #[allow(clippy::new_ret_no_self)] // intentional: returns a builder
    pub fn new(name: impl Into<String>) -> PackageBuilder {
        PackageBuilder::new(name)
    }

    /// Publish assets of the given type to the destination
    pub async fn publish(
        &self,
        asset_type: AssetType,
        dest: impl AsRef<Path>,
    ) -> PackageResult<()> {
        let publisher = Publisher::new(self.clone());
        publisher.publish(asset_type, dest).await
    }

    /// Publish assets with the given tag
    pub async fn publish_tag(&self, tag: &str, base_dest: impl AsRef<Path>) -> PackageResult<()> {
        let asset_types = self
            .tags
            .get(tag)
            .ok_or_else(|| PackageError::PublishError(format!("Tag '{}' not found", tag)))?;

        for asset_type in asset_types {
            let dest = base_dest.as_ref().join(asset_type.default_path());
            self.publish(asset_type.clone(), dest).await?;
        }

        Ok(())
    }

    /// Validate package name
    fn validate_name(name: &str) -> PackageResult<()> {
        if name.is_empty() {
            return Err(PackageError::NameRequired);
        }

        // Package name should be kebab-case
        let valid = name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid {
            return Err(PackageError::InvalidName(
                "Package name must be lowercase alphanumeric with hyphens".to_string(),
            ));
        }

        Ok(())
    }
}

/// Builder for creating packages
pub struct PackageBuilder {
    name: String,
    description: Option<String>,
    author: Option<String>,
    version: String,
    base_path: PathBuf,
    configs: Vec<PathBuf>,
    migrations: Vec<PathBuf>,
    views: Vec<PathBuf>,
    assets: HashMap<String, PathBuf>,
    tags: HashMap<String, Vec<AssetType>>,
}

impl PackageBuilder {
    /// Create a new package builder
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            base_path: PathBuf::from(format!("packages/{}", name)),
            name,
            description: None,
            author: None,
            version: "0.1.0".to_string(),
            configs: Vec::new(),
            migrations: Vec::new(),
            views: Vec::new(),
            assets: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    /// Set package description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set package author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set package version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set base path
    pub fn base_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_path = path.into();
        self
    }

    /// Add a config file
    pub fn config(mut self, path: impl Into<PathBuf>) -> Self {
        self.configs.push(self.base_path.join(path.into()));
        self
    }

    /// Add a migration
    pub fn migration(mut self, name: impl AsRef<str>) -> Self {
        let path = self
            .base_path
            .join("migrations")
            .join(format!("{}.sql", name.as_ref()));
        self.migrations.push(path);
        self
    }

    /// Add a view directory
    pub fn view(mut self, path: impl Into<PathBuf>) -> Self {
        self.views.push(self.base_path.join(path.into()));
        self
    }

    /// Add a custom asset
    pub fn asset(mut self, key: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        self.assets
            .insert(key.into(), self.base_path.join(path.into()));
        self
    }

    /// Add a tag for grouping assets
    pub fn tag(mut self, tag: impl Into<String>, assets: Vec<AssetType>) -> Self {
        self.tags.insert(tag.into(), assets);
        self
    }

    /// Build the package
    pub fn build(self) -> PackageResult<Package> {
        Package::validate_name(&self.name)?;

        Ok(Package {
            name: self.name,
            description: self.description,
            author: self.author,
            version: self.version,
            base_path: self.base_path,
            configs: self.configs,
            migrations: self.migrations,
            views: self.views,
            assets: self.assets,
            tags: self.tags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_builder() {
        let package = Package::new("my-package")
            .description("Test package")
            .author("Test Author")
            .version("1.0.0")
            .config("config/test.toml")
            .migration("create_users")
            .view("templates")
            .build()
            .unwrap();

        assert_eq!(package.name, "my-package");
        assert_eq!(package.description, Some("Test package".to_string()));
        assert_eq!(package.version, "1.0.0");
        assert_eq!(package.configs.len(), 1);
        assert_eq!(package.migrations.len(), 1);
        assert_eq!(package.views.len(), 1);
    }

    #[test]
    fn test_validate_name() {
        assert!(Package::validate_name("my-package").is_ok());
        assert!(Package::validate_name("my-package-123").is_ok());
        assert!(Package::validate_name("").is_err());
        assert!(Package::validate_name("My-Package").is_err());
        assert!(Package::validate_name("my_package").is_err());
    }

    #[test]
    fn test_tags() {
        let package = Package::new("my-package")
            .tag("public", vec![AssetType::Config, AssetType::Views])
            .tag("migrations", vec![AssetType::Migrations])
            .build()
            .unwrap();

        assert_eq!(package.tags.len(), 2);
        assert_eq!(package.tags.get("public").unwrap().len(), 2);
    }

    #[test]
    fn test_custom_assets() {
        let package = Package::new("my-package")
            .asset("styles", "assets/styles.css")
            .asset("scripts", "assets/app.js")
            .build()
            .unwrap();

        assert_eq!(package.assets.len(), 2);
        assert!(package.assets.contains_key("styles"));
        assert!(package.assets.contains_key("scripts"));
    }
}
