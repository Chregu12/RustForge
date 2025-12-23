//! Global storage manager for facade pattern

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global storage manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_STORAGE: Lazy<RwLock<StorageManagerFacade>> = Lazy::new(|| {
    RwLock::new(StorageManagerFacade::new())
});

/// Storage manager that holds file storage state (for facade pattern)
pub struct StorageManagerFacade {
    /// In-memory file storage (path -> contents)
    files: HashMap<String, Vec<u8>>,
    /// Current disk name
    disk: String,
}

impl StorageManagerFacade {
    /// Create a new storage manager
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            disk: "local".to_string(),
        }
    }

    /// Put a file
    pub fn put(&mut self, path: &str, contents: Vec<u8>) -> Result<(), String> {
        self.files.insert(path.to_string(), contents);
        Ok(())
    }

    /// Get a file
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| format!("File not found: {}", path))
    }

    /// Check if a file exists
    pub fn exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    /// Delete a file
    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        self.files
            .remove(path)
            .ok_or_else(|| format!("File not found: {}", path))?;
        Ok(())
    }

    /// Get the size of a file
    pub fn size(&self, path: &str) -> Result<u64, String> {
        self.files
            .get(path)
            .map(|contents| contents.len() as u64)
            .ok_or_else(|| format!("File not found: {}", path))
    }

    /// List all files
    pub fn files(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }

    /// List all files in a directory
    pub fn files_in(&self, directory: &str) -> Vec<String> {
        self.files
            .keys()
            .filter(|path| path.starts_with(directory))
            .cloned()
            .collect()
    }

    /// Get all directories
    pub fn directories(&self) -> Vec<String> {
        let mut dirs = std::collections::HashSet::new();
        for path in self.files.keys() {
            if let Some(pos) = path.rfind('/') {
                let dir = &path[..pos];
                dirs.insert(dir.to_string());
            }
        }
        dirs.into_iter().collect()
    }

    /// Copy a file
    pub fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        let contents = self.get(from)?;
        self.put(to, contents)?;
        Ok(())
    }

    /// Move a file
    pub fn move_file(&mut self, from: &str, to: &str) -> Result<(), String> {
        let contents = self.get(from)?;
        self.delete(from)?;
        self.put(to, contents)?;
        Ok(())
    }

    /// Get the current disk name
    pub fn disk_name(&self) -> &str {
        &self.disk
    }

    /// Set the disk name
    pub fn set_disk(&mut self, disk: String) {
        self.disk = disk;
    }

    /// Prepend content to a file
    pub fn prepend(&mut self, path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut contents = self.get(path).unwrap_or_default();
        let mut new_contents = data;
        new_contents.append(&mut contents);
        self.put(path, new_contents)?;
        Ok(())
    }

    /// Append content to a file
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
