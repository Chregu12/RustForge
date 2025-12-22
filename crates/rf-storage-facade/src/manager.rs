//! Global storage manager

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global storage manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_STORAGE: Lazy<RwLock<StorageManager>> = Lazy::new(|| {
    RwLock::new(StorageManager::new())
});

/// Storage manager that holds file storage state
pub struct StorageManager {
    /// In-memory file storage (path -> contents)
    files: HashMap<String, Vec<u8>>,
    /// Current disk name
    disk: String,
}

impl StorageManager {
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

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_manager_new() {
        let manager = StorageManager::new();
        assert_eq!(manager.files().len(), 0);
        assert_eq!(manager.disk_name(), "local");
    }

    #[test]
    fn test_storage_manager_put_get() {
        let mut manager = StorageManager::new();
        let contents = b"Hello, World!".to_vec();

        manager.put("file.txt", contents.clone()).unwrap();
        let retrieved = manager.get("file.txt").unwrap();

        assert_eq!(retrieved, contents);
    }

    #[test]
    fn test_storage_manager_exists() {
        let mut manager = StorageManager::new();
        assert!(!manager.exists("file.txt"));

        manager.put("file.txt", b"test".to_vec()).unwrap();
        assert!(manager.exists("file.txt"));
    }

    #[test]
    fn test_storage_manager_delete() {
        let mut manager = StorageManager::new();
        manager.put("file.txt", b"test".to_vec()).unwrap();
        assert!(manager.exists("file.txt"));

        manager.delete("file.txt").unwrap();
        assert!(!manager.exists("file.txt"));
    }

    #[test]
    fn test_storage_manager_size() {
        let mut manager = StorageManager::new();
        let contents = b"Hello, World!".to_vec();
        manager.put("file.txt", contents.clone()).unwrap();

        let size = manager.size("file.txt").unwrap();
        assert_eq!(size, contents.len() as u64);
    }

    #[test]
    fn test_storage_manager_files() {
        let mut manager = StorageManager::new();
        manager.put("file1.txt", b"test1".to_vec()).unwrap();
        manager.put("file2.txt", b"test2".to_vec()).unwrap();

        let files = manager.files();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"file1.txt".to_string()));
        assert!(files.contains(&"file2.txt".to_string()));
    }

    #[test]
    fn test_storage_manager_files_in() {
        let mut manager = StorageManager::new();
        manager.put("dir1/file1.txt", b"test1".to_vec()).unwrap();
        manager.put("dir1/file2.txt", b"test2".to_vec()).unwrap();
        manager.put("dir2/file3.txt", b"test3".to_vec()).unwrap();

        let files = manager.files_in("dir1");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_storage_manager_copy() {
        let mut manager = StorageManager::new();
        manager.put("file1.txt", b"test".to_vec()).unwrap();

        manager.copy("file1.txt", "file2.txt").unwrap();

        assert!(manager.exists("file1.txt"));
        assert!(manager.exists("file2.txt"));
        assert_eq!(manager.get("file1.txt").unwrap(), manager.get("file2.txt").unwrap());
    }

    #[test]
    fn test_storage_manager_move() {
        let mut manager = StorageManager::new();
        let contents = b"test".to_vec();
        manager.put("file1.txt", contents.clone()).unwrap();

        manager.move_file("file1.txt", "file2.txt").unwrap();

        assert!(!manager.exists("file1.txt"));
        assert!(manager.exists("file2.txt"));
        assert_eq!(manager.get("file2.txt").unwrap(), contents);
    }

    #[test]
    fn test_storage_manager_disk() {
        let mut manager = StorageManager::new();
        assert_eq!(manager.disk_name(), "local");

        manager.set_disk("s3".to_string());
        assert_eq!(manager.disk_name(), "s3");
    }

    #[test]
    fn test_storage_manager_append() {
        let mut manager = StorageManager::new();
        manager.put("file.txt", b"Hello".to_vec()).unwrap();
        manager.append("file.txt", b", World!".to_vec()).unwrap();

        let contents = manager.get("file.txt").unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[test]
    fn test_storage_manager_prepend() {
        let mut manager = StorageManager::new();
        manager.put("file.txt", b"World!".to_vec()).unwrap();
        manager.prepend("file.txt", b"Hello, ".to_vec()).unwrap();

        let contents = manager.get("file.txt").unwrap();
        assert_eq!(contents, b"Hello, World!");
    }
}
