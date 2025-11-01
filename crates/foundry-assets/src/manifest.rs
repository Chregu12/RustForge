//! Asset manifest generation

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Entry in asset manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    /// Original file path
    pub original: String,
    /// Versioned file path
    pub versioned: String,
    /// Content hash
    pub hash: String,
    /// File size in bytes
    pub size: u64,
}

/// Asset manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetManifest {
    /// Map of original path to asset entry
    pub assets: HashMap<String, AssetEntry>,
    /// Generation timestamp
    pub generated_at: String,
}

impl AssetManifest {
    /// Create a new manifest
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add asset to manifest
    pub fn add_asset(&mut self, original: String, versioned: String, hash: String, size: u64) {
        self.assets.insert(
            original.clone(),
            AssetEntry {
                original,
                versioned,
                hash,
                size,
            },
        );
    }

    /// Get versioned path for original
    pub fn get_versioned(&self, original: &str) -> Option<&str> {
        self.assets.get(original).map(|e| e.versioned.as_str())
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Load manifest from file
    pub fn load(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)?;
        let manifest: AssetManifest = serde_json::from_str(&json)?;
        Ok(manifest)
    }

    /// Get total size of all assets
    pub fn total_size(&self) -> u64 {
        self.assets.values().map(|e| e.size).sum()
    }

    /// Get asset count
    pub fn count(&self) -> usize {
        self.assets.len()
    }
}

impl Default for AssetManifest {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_new() {
        let manifest = AssetManifest::new();
        assert_eq!(manifest.count(), 0);
        assert!(!manifest.generated_at.is_empty());
    }

    #[test]
    fn test_add_asset() {
        let mut manifest = AssetManifest::new();
        manifest.add_asset(
            "app.js".to_string(),
            "app.abc123.js".to_string(),
            "abc123".to_string(),
            1024,
        );

        assert_eq!(manifest.count(), 1);
        assert_eq!(manifest.get_versioned("app.js"), Some("app.abc123.js"));
    }

    #[test]
    fn test_total_size() {
        let mut manifest = AssetManifest::new();
        manifest.add_asset("file1".to_string(), "file1.v".to_string(), "h1".to_string(), 100);
        manifest.add_asset("file2".to_string(), "file2.v".to_string(), "h2".to_string(), 200);

        assert_eq!(manifest.total_size(), 300);
    }

    #[test]
    fn test_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("manifest.json");

        let mut manifest = AssetManifest::new();
        manifest.add_asset(
            "test.js".to_string(),
            "test.abc.js".to_string(),
            "abc".to_string(),
            512,
        );

        manifest.save(&manifest_path).unwrap();
        assert!(manifest_path.exists());

        let loaded = AssetManifest::load(&manifest_path).unwrap();
        assert_eq!(loaded.count(), 1);
        assert_eq!(loaded.get_versioned("test.js"), Some("test.abc.js"));
    }
}
