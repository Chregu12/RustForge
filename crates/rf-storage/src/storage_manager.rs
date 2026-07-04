//! Global storage manager for facade pattern.
//!
//! Backs the synchronous [`crate::StorageFacade`]. Unlike the async
//! [`crate::LocalStorage`] engine (which uses `tokio::fs`), this manager uses
//! blocking `std::fs` so the facade can offer a Laravel-style, `.await`-free API
//! without any sync-over-async deadlock risk.
//!
//! Files are persisted to the local filesystem under a configurable base
//! directory (default `./storage/app`, overridable via the `RF_STORAGE_ROOT`
//! environment variable).

use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Default base directory for the local disk when no override is provided.
pub const DEFAULT_STORAGE_ROOT: &str = "./storage/app";

/// Environment variable used to override the base storage directory.
pub const STORAGE_ROOT_ENV: &str = "RF_STORAGE_ROOT";

/// Global storage manager instance.
///
/// Uses `std::sync::RwLock` for synchronous access (no `.await` needed) and
/// persists to real files on the local filesystem.
pub static GLOBAL_STORAGE: Lazy<RwLock<StorageManagerFacade>> =
    Lazy::new(|| RwLock::new(StorageManagerFacade::new()));

/// Storage manager that persists files to the local filesystem (facade pattern).
pub struct StorageManagerFacade {
    /// Base directory that all relative paths are resolved against.
    root: PathBuf,
    /// Current disk name.
    disk: String,
}

impl StorageManagerFacade {
    /// Create a new storage manager rooted at the default (or env-overridden) base directory.
    pub fn new() -> Self {
        let root = std::env::var(STORAGE_ROOT_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_STORAGE_ROOT));

        Self {
            root,
            disk: "local".to_string(),
        }
    }

    /// Get the base directory files are stored under.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Set the base directory files are stored under.
    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
    }

    /// Resolve a logical path to a full filesystem path, rejecting traversal.
    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let normalized = path.trim_start_matches('/');

        // Security: reject path traversal.
        if normalized.split(['/', '\\']).any(|c| c == "..") {
            return Err(format!("Invalid path: path traversal detected in '{}'", path));
        }

        Ok(self.root.join(normalized))
    }

    /// Convert a path relative to `root` into a forward-slash string key.
    fn rel_key(root: &Path, full: &Path) -> Option<String> {
        let rel = full.strip_prefix(root).ok()?;
        let parts: Vec<String> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
            .collect();
        Some(parts.join("/"))
    }

    /// Recursively collect all files under `dir` as keys relative to `root`.
    fn walk_files(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                Self::walk_files(&p, root, out);
            } else if p.is_file() {
                if let Some(key) = Self::rel_key(root, &p) {
                    out.push(key);
                }
            }
        }
    }

    /// Recursively collect all directories under `dir` as keys relative to `root`.
    fn walk_dirs(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(key) = Self::rel_key(root, &p) {
                    out.push(key);
                }
                Self::walk_dirs(&p, root, out);
            }
        }
    }

    /// Put a file (writes it to disk, creating parent directories as needed).
    pub fn put(&mut self, path: &str, contents: Vec<u8>) -> Result<(), String> {
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {:?}: {}", parent, e))?;
        }
        std::fs::write(&full, contents)
            .map_err(|e| format!("Failed to write file {}: {}", path, e))
    }

    /// Get a file's contents from disk.
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        let full = self.resolve(path)?;
        std::fs::read(&full).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("File not found: {}", path)
            } else {
                format!("Failed to read file {}: {}", path, e)
            }
        })
    }

    /// Check if a file exists on disk.
    pub fn exists(&self, path: &str) -> bool {
        match self.resolve(path) {
            Ok(full) => full.is_file(),
            Err(_) => false,
        }
    }

    /// Delete a file from disk.
    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        let full = self.resolve(path)?;
        if !full.exists() {
            return Err(format!("File not found: {}", path));
        }
        std::fs::remove_file(&full).map_err(|e| format!("Failed to delete file {}: {}", path, e))
    }

    /// Get the size of a file in bytes.
    pub fn size(&self, path: &str) -> Result<u64, String> {
        let full = self.resolve(path)?;
        let meta = std::fs::metadata(&full).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("File not found: {}", path)
            } else {
                format!("Failed to stat file {}: {}", path, e)
            }
        })?;
        Ok(meta.len())
    }

    /// List all files under the base directory (recursively).
    pub fn files(&self) -> Vec<String> {
        let mut out = Vec::new();
        Self::walk_files(&self.root, &self.root, &mut out);
        out
    }

    /// List all files under a directory (recursively).
    pub fn files_in(&self, directory: &str) -> Vec<String> {
        let dir = match self.resolve(directory) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        Self::walk_files(&dir, &self.root, &mut out);
        out
    }

    /// Get all directories under the base directory (recursively).
    pub fn directories(&self) -> Vec<String> {
        let mut out = Vec::new();
        Self::walk_dirs(&self.root, &self.root, &mut out);
        out
    }

    /// Copy a file.
    pub fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        let contents = self.get(from)?;
        self.put(to, contents)?;
        Ok(())
    }

    /// Move a file.
    pub fn move_file(&mut self, from: &str, to: &str) -> Result<(), String> {
        let contents = self.get(from)?;
        self.put(to, contents)?;
        self.delete(from)?;
        Ok(())
    }

    /// Get the current disk name.
    pub fn disk_name(&self) -> &str {
        &self.disk
    }

    /// Set the disk name.
    pub fn set_disk(&mut self, disk: String) {
        self.disk = disk;
    }

    /// Prepend content to a file.
    pub fn prepend(&mut self, path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut contents = self.get(path).unwrap_or_default();
        let mut new_contents = data;
        new_contents.append(&mut contents);
        self.put(path, new_contents)?;
        Ok(())
    }

    /// Append content to a file.
    pub fn append(&mut self, path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut contents = self.get(path).unwrap_or_default();
        contents.extend(data);
        self.put(path, contents)?;
        Ok(())
    }
}

impl Default for StorageManagerFacade {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_manager() -> StorageManagerFacade {
        let dir = std::env::temp_dir().join(format!(
            "rf_storage_mgr_{}_{}",
            std::process::id(),
            // unique per test to avoid cross-test interference
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut m = StorageManagerFacade::new();
        m.set_root(&dir);
        m
    }

    #[test]
    fn test_put_get_delete_on_disk() {
        let mut m = temp_manager();
        let root = m.root().to_path_buf();

        m.put("docs/hello.txt", b"hi".to_vec()).unwrap();

        // Actually on disk.
        assert!(root.join("docs").join("hello.txt").is_file());
        assert!(m.exists("docs/hello.txt"));
        assert_eq!(m.get("docs/hello.txt").unwrap(), b"hi");
        assert_eq!(m.size("docs/hello.txt").unwrap(), 2);

        m.delete("docs/hello.txt").unwrap();
        assert!(!m.exists("docs/hello.txt"));
        assert!(!root.join("docs").join("hello.txt").exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_missing_file_errors() {
        let m = temp_manager();
        assert!(m.get("nope.txt").is_err());
        assert!(!m.exists("nope.txt"));
        let _ = std::fs::remove_dir_all(m.root());
    }

    #[test]
    fn test_path_traversal_rejected() {
        let mut m = temp_manager();
        assert!(m.put("../escape.txt", b"x".to_vec()).is_err());
        assert!(m.get("../../etc/passwd").is_err());
        let _ = std::fs::remove_dir_all(m.root());
    }

    #[test]
    fn test_files_listing() {
        let mut m = temp_manager();
        m.put("a.txt", b"1".to_vec()).unwrap();
        m.put("sub/b.txt", b"2".to_vec()).unwrap();

        let mut files = m.files();
        files.sort();
        assert_eq!(files, vec!["a.txt".to_string(), "sub/b.txt".to_string()]);

        let in_sub = m.files_in("sub");
        assert_eq!(in_sub, vec!["sub/b.txt".to_string()]);

        assert_eq!(m.directories(), vec!["sub".to_string()]);
        let _ = std::fs::remove_dir_all(m.root());
    }

    #[test]
    fn test_copy_move_append_prepend() {
        let mut m = temp_manager();
        m.put("src.txt", b"data".to_vec()).unwrap();

        m.copy("src.txt", "copy.txt").unwrap();
        assert_eq!(m.get("copy.txt").unwrap(), b"data");
        assert!(m.exists("src.txt"));

        m.move_file("src.txt", "moved.txt").unwrap();
        assert!(!m.exists("src.txt"));
        assert_eq!(m.get("moved.txt").unwrap(), b"data");

        m.append("moved.txt", b"!".to_vec()).unwrap();
        m.prepend("moved.txt", b">".to_vec()).unwrap();
        assert_eq!(m.get("moved.txt").unwrap(), b">data!");

        let _ = std::fs::remove_dir_all(m.root());
    }
}
