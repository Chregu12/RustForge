//! Asset content hashing for cache busting

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Asset hasher for cache busting
pub struct AssetHasher;

impl AssetHasher {
    /// Generate hash for file content
    pub fn hash_file(path: &Path) -> Result<String> {
        let content = fs::read(path)?;
        let hash = Self::hash_bytes(&content);
        Ok(hash)
    }

    /// Generate hash from bytes
    pub fn hash_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generate short hash (first 8 characters)
    pub fn short_hash(hash: &str) -> String {
        hash.chars().take(8).collect()
    }

    /// Generate versioned filename
    pub fn versioned_filename(filename: &str, hash: &str) -> String {
        let short = Self::short_hash(hash);

        if let Some((name, ext)) = filename.rsplit_once('.') {
            format!("{}.{}.{}", name, short, ext)
        } else {
            format!("{}.{}", filename, short)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_bytes() {
        let content = b"hello world";
        let hash = AssetHasher::hash_bytes(content);
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }

    #[test]
    fn test_hash_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        let hash = AssetHasher::hash_file(&file_path).unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_short_hash() {
        let hash = "abcdef1234567890";
        let short = AssetHasher::short_hash(hash);
        assert_eq!(short, "abcdef12");
    }

    #[test]
    fn test_versioned_filename() {
        let hash = "abcdef1234567890";
        let filename = AssetHasher::versioned_filename("app.js", hash);
        assert_eq!(filename, "app.abcdef12.js");

        let filename_no_ext = AssetHasher::versioned_filename("README", hash);
        assert_eq!(filename_no_ext, "README.abcdef12");
    }
}
