//! Asset publishing logic

use crate::hasher::AssetHasher;
use crate::manifest::AssetManifest;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration for asset publishing
#[derive(Debug, Clone)]
pub struct PublishConfig {
    /// Source directory
    pub source_dir: PathBuf,
    /// Target directory
    pub target_dir: PathBuf,
    /// Whether to use versioned filenames
    pub versioning: bool,
    /// File extensions to include
    pub include_extensions: Vec<String>,
    /// Directories to exclude
    pub exclude_dirs: Vec<String>,
}

impl Default for PublishConfig {
    fn default() -> Self {
        Self {
            source_dir: PathBuf::from("assets"),
            target_dir: PathBuf::from("public"),
            versioning: true,
            include_extensions: vec![
                "js".to_string(),
                "css".to_string(),
                "png".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "svg".to_string(),
                "woff".to_string(),
                "woff2".to_string(),
                "ttf".to_string(),
            ],
            exclude_dirs: vec![".git".to_string(), "node_modules".to_string()],
        }
    }
}

/// Result of asset publishing
#[derive(Debug, Clone)]
pub struct PublishResult {
    /// Number of files published
    pub files_published: usize,
    /// Total bytes copied
    pub bytes_copied: u64,
    /// Asset manifest
    pub manifest: AssetManifest,
}

/// Asset publisher
pub struct AssetPublisher {
    config: PublishConfig,
}

impl AssetPublisher {
    /// Create a new asset publisher
    pub fn new(config: PublishConfig) -> Self {
        Self { config }
    }

    /// Publish all assets
    pub fn publish(&self) -> Result<PublishResult> {
        // Create target directory if it doesn't exist
        if !self.config.target_dir.exists() {
            fs::create_dir_all(&self.config.target_dir)
                .context("Failed to create target directory")?;
        }

        let mut manifest = AssetManifest::new();
        let mut files_published = 0;
        let mut bytes_copied = 0;

        // Walk source directory
        for entry in WalkDir::new(&self.config.source_dir)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !self.is_excluded(e.path()))
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Check extension
            if !self.should_include(path) {
                continue;
            }

            // Publish file
            let relative_path = path
                .strip_prefix(&self.config.source_dir)
                .context("Failed to get relative path")?;

            let (target_path, hash) = if self.config.versioning {
                let hash = AssetHasher::hash_file(path)?;
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("Invalid filename")?;
                let versioned = AssetHasher::versioned_filename(filename, &hash);

                let mut target = self.config.target_dir.clone();
                if let Some(parent) = relative_path.parent() {
                    target.push(parent);
                }
                target.push(versioned);

                (target, Some(hash))
            } else {
                (self.config.target_dir.join(relative_path), None)
            };

            // Create parent directory
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Copy file
            fs::copy(path, &target_path)?;

            let metadata = fs::metadata(&target_path)?;
            let size = metadata.len();

            bytes_copied += size;
            files_published += 1;

            // Add to manifest
            if let Some(hash) = hash {
                let original = relative_path.to_string_lossy().to_string();
                let versioned = target_path
                    .strip_prefix(&self.config.target_dir)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                manifest.add_asset(original, versioned, hash, size);
            }
        }

        Ok(PublishResult {
            files_published,
            bytes_copied,
            manifest,
        })
    }

    /// Check if path should be excluded
    fn is_excluded(&self, path: &Path) -> bool {
        path.components().any(|c| {
            if let Some(name) = c.as_os_str().to_str() {
                self.config.exclude_dirs.contains(&name.to_string())
            } else {
                false
            }
        })
    }

    /// Check if file should be included
    fn should_include(&self, path: &Path) -> bool {
        if self.config.include_extensions.is_empty() {
            return true;
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.config.include_extensions.contains(&ext.to_string())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = PublishConfig::default();
        assert_eq!(config.source_dir, PathBuf::from("assets"));
        assert!(config.versioning);
    }

    #[test]
    fn test_publisher_new() {
        let config = PublishConfig::default();
        let publisher = AssetPublisher::new(config);
        assert!(publisher.config.versioning);
    }

    #[test]
    fn test_publish() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("assets");
        let target_dir = temp_dir.path().join("public");

        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("app.js"), "console.log('test');").unwrap();
        fs::write(source_dir.join("style.css"), "body { margin: 0; }").unwrap();

        let config = PublishConfig {
            source_dir,
            target_dir: target_dir.clone(),
            versioning: true,
            include_extensions: vec!["js".to_string(), "css".to_string()],
            exclude_dirs: vec![],
        };

        let publisher = AssetPublisher::new(config);
        let result = publisher.publish().unwrap();

        assert_eq!(result.files_published, 2);
        assert!(result.bytes_copied > 0);
        assert_eq!(result.manifest.count(), 2);
    }

    #[test]
    fn test_is_excluded() {
        let config = PublishConfig {
            exclude_dirs: vec!["node_modules".to_string()],
            ..Default::default()
        };

        let publisher = AssetPublisher::new(config);

        assert!(publisher.is_excluded(Path::new("src/node_modules/package")));
        assert!(!publisher.is_excluded(Path::new("src/components/Button.js")));
    }

    #[test]
    fn test_should_include() {
        let config = PublishConfig {
            include_extensions: vec!["js".to_string(), "css".to_string()],
            ..Default::default()
        };

        let publisher = AssetPublisher::new(config);

        assert!(publisher.should_include(Path::new("app.js")));
        assert!(publisher.should_include(Path::new("style.css")));
        assert!(!publisher.should_include(Path::new("README.md")));
    }
}
