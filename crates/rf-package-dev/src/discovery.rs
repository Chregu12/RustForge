//! Package auto-discovery

use crate::{Package, PackageError, PackageResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Base directories to search for packages
    pub search_paths: Vec<PathBuf>,

    /// Package name patterns (regex)
    pub patterns: Vec<String>,

    /// Whether to search recursively
    pub recursive: bool,

    /// Maximum depth for recursive search
    pub max_depth: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![PathBuf::from("packages"), PathBuf::from("vendor")],
            patterns: vec![r"^[a-z0-9-]+$".to_string()],
            recursive: true,
            max_depth: 3,
        }
    }
}

/// Package discovery system
pub struct Discovery {
    config: DiscoveryConfig,
}

impl Discovery {
    /// Create a new discovery instance with default config
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
        }
    }

    /// Create a new discovery instance with custom config
    pub fn with_config(config: DiscoveryConfig) -> Self {
        Self { config }
    }

    /// Discover all packages in the configured search paths
    pub async fn discover_all(&self) -> PackageResult<Vec<Package>> {
        let mut packages = Vec::new();

        for search_path in &self.config.search_paths {
            if !search_path.exists() {
                continue;
            }

            let discovered = self.discover_in_path(search_path, 0).await?;
            packages.extend(discovered);
        }

        Ok(packages)
    }

    /// Discover packages in a specific path
    fn discover_in_path<'a>(
        &'a self,
        path: &'a Path,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = PackageResult<Vec<Package>>> + 'a>>
    {
        Box::pin(async move {
            if depth > self.config.max_depth {
                return Ok(Vec::new());
            }

            let mut packages = Vec::new();

            // Check if this directory contains a package manifest
            if let Some(package) = self.load_package(path).await? {
                packages.push(package);
            }

            // Search subdirectories if recursive
            if self.config.recursive {
                let entries = std::fs::read_dir(path)?;
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_dir() {
                        let discovered = self.discover_in_path(&path, depth + 1).await?;
                        packages.extend(discovered);
                    }
                }
            }

            Ok(packages)
        })
    }

    /// Load a package from a directory
    async fn load_package(&self, path: &Path) -> PackageResult<Option<Package>> {
        // Look for package manifest files
        let manifest_names = ["package.toml", "Package.toml", "rf-package.toml"];

        for name in &manifest_names {
            let manifest_path = path.join(name);
            if manifest_path.exists() {
                return Ok(Some(self.load_from_manifest(&manifest_path).await?));
            }
        }

        Ok(None)
    }

    /// Load package from a manifest file
    async fn load_from_manifest(&self, manifest_path: &Path) -> PackageResult<Package> {
        let contents = std::fs::read_to_string(manifest_path)?;
        let package: Package = toml::from_str(&contents)
            .map_err(|e| PackageError::PublishError(format!("Failed to parse manifest: {}", e)))?;

        Ok(package)
    }

    /// Validate package name against patterns
    pub fn validate_package_name(&self, name: &str) -> bool {
        for pattern in &self.config.patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(name) {
                    return true;
                }
            }
        }
        false
    }

    /// Find a specific package by name
    pub async fn find_package(&self, name: &str) -> PackageResult<Option<Package>> {
        let packages = self.discover_all().await?;
        Ok(packages.into_iter().find(|p| p.name == name))
    }
}

impl Default for Discovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.search_paths.len(), 2);
        assert!(config.recursive);
        assert_eq!(config.max_depth, 3);
    }

    #[test]
    fn test_validate_package_name() {
        let discovery = Discovery::new();
        assert!(discovery.validate_package_name("my-package"));
        assert!(discovery.validate_package_name("my-package-123"));
        assert!(!discovery.validate_package_name("My-Package"));
        assert!(!discovery.validate_package_name("my_package"));
    }

    #[tokio::test]
    async fn test_discover_empty() {
        let config = DiscoveryConfig {
            search_paths: vec![PathBuf::from("/nonexistent/path")],
            patterns: vec![r"^[a-z0-9-]+$".to_string()],
            recursive: true,
            max_depth: 3,
        };

        let discovery = Discovery::with_config(config);
        let packages = discovery.discover_all().await.unwrap();
        assert_eq!(packages.len(), 0);
    }
}
