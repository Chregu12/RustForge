//! Storage facade providing Laravel-style static storage API

use crate::storage_manager::GLOBAL_STORAGE;

/// The StorageFacade providing a static-like API for file storage.
///
/// Simple, Laravel-style API - no `.await` needed anywhere!
///
/// # Examples
///
/// ```rust,no_run
/// use rf_storage::StorageFacade;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Put a file
/// StorageFacade::put("file.txt", b"Hello, World!".to_vec())?;
///
/// // Get a file
/// let contents = StorageFacade::get("file.txt")?;
///
/// // Check if exists
/// if StorageFacade::exists("file.txt") {
///     println!("File exists!");
/// }
///
/// // Delete file
/// StorageFacade::delete("file.txt")?;
/// # Ok(())
/// # }
/// ```
pub struct StorageFacade;

impl StorageFacade {
    /// Put a file
    pub fn put(path: &str, contents: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.put(path, contents)
    }

    /// Get a file's contents
    pub fn get(path: &str) -> Result<Vec<u8>, String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.get(path)
    }

    /// Get a file's contents as a string
    pub fn get_string(path: &str) -> Result<String, String> {
        let contents = Self::get(path)?;
        String::from_utf8(contents)
            .map_err(|e| format!("Failed to convert file to UTF-8: {}", e))
    }

    /// Check if a file exists
    pub fn exists(path: &str) -> bool {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.exists(path)
    }

    /// Delete a file
    pub fn delete(path: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.delete(path)
    }

    /// Alias for [`delete`] — naming-consistency convenience.
    ///
    /// [`delete`]: StorageFacade::delete
    pub fn forget(path: &str) -> Result<(), String> {
        Self::delete(path)
    }

    /// Get the size of a file
    pub fn size(path: &str) -> Result<u64, String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.size(path)
    }

    /// List all files
    pub fn files() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.files()
    }

    /// List all files in a directory
    pub fn files_in(directory: &str) -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.files_in(directory)
    }

    /// Get all directories
    pub fn directories() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.directories()
    }

    /// Copy a file
    pub fn copy(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.copy(from, to)
    }

    /// Move a file
    pub fn move_file(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.move_file(from, to)
    }

    /// Prepend content to a file
    pub fn prepend(path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.prepend(path, data)
    }

    /// Append content to a file
    pub fn append(path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.append(path, data)
    }

    /// Get the current disk name
    pub fn disk_name() -> String {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.disk_name().to_string()
    }

    /// Set the disk to use
    pub fn disk(name: &str) {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.set_disk(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_put_get() {
        let contents = b"Hello, World!".to_vec();
        StorageFacade::put("test_facade_put.txt", contents.clone()).unwrap();

        let retrieved = StorageFacade::get("test_facade_put.txt").unwrap();
        assert_eq!(retrieved, contents);
    }

    #[test]
    fn test_storage_exists() {
        let path = "test_facade_exists.txt";
        StorageFacade::put(path, b"test".to_vec()).unwrap();

        assert!(StorageFacade::exists(path));
        assert!(!StorageFacade::exists("nonexistent_facade.txt"));
    }

    #[test]
    fn test_storage_delete() {
        let path = "test_facade_delete.txt";
        StorageFacade::put(path, b"test".to_vec()).unwrap();
        assert!(StorageFacade::exists(path));

        StorageFacade::delete(path).unwrap();
        assert!(!StorageFacade::exists(path));
    }
}
