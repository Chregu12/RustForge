//! Storage facade providing Laravel-style static storage API
//!
//! All methods are simple to use - no `.await` needed!

use crate::manager::GLOBAL_STORAGE;

/// The Storage facade providing a static-like API for file storage.
///
/// Simple, Laravel-style API - no `.await` needed anywhere!
///
/// # Examples
///
/// ```rust,no_run
/// use rf_storage_facade::Storage;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Put a file
/// Storage::put("file.txt", b"Hello, World!".to_vec())?;
///
/// // Get a file
/// let contents = Storage::get("file.txt")?;
/// println!("File contents: {:?}", String::from_utf8_lossy(&contents));
///
/// // Check if exists
/// if Storage::exists("file.txt") {
///     println!("File exists!");
/// }
///
/// // Get file size
/// let size = Storage::size("file.txt")?;
/// println!("File size: {} bytes", size);
///
/// // Delete file
/// Storage::delete("file.txt")?;
/// # Ok(())
/// # }
/// ```
pub struct Storage;

impl Storage {
    /// Put a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::put("file.txt", b"Hello, World!".to_vec())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn put(path: &str, contents: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.put(path, contents)
    }

    /// Get a file's contents
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contents = Storage::get("file.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(path: &str) -> Result<Vec<u8>, String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.get(path)
    }

    /// Get a file's contents as a string
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let text = Storage::get_string("file.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_string(path: &str) -> Result<String, String> {
        let contents = Self::get(path)?;
        String::from_utf8(contents)
            .map_err(|e| format!("Failed to convert file to UTF-8: {}", e))
    }

    /// Check if a file exists
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() {
    /// if Storage::exists("file.txt") {
    ///     println!("File exists!");
    /// }
    /// # }
    /// ```
    pub fn exists(path: &str) -> bool {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.exists(path)
    }

    /// Delete a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::delete("file.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete(path: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.delete(path)
    }

    /// Alias for [`delete`] — naming-consistency convenience.
    ///
    /// [`delete`]: Storage::delete
    pub fn forget(path: &str) -> Result<(), String> {
        Self::delete(path)
    }

    /// Get the size of a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let size = Storage::size("file.txt")?;
    /// println!("File size: {} bytes", size);
    /// # Ok(())
    /// # }
    /// ```
    pub fn size(path: &str) -> Result<u64, String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.size(path)
    }

    /// List all files
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() {
    /// let files = Storage::files();
    /// for file in files {
    ///     println!("File: {}", file);
    /// }
    /// # }
    /// ```
    pub fn files() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.files()
    }

    /// List all files in a directory
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() {
    /// let files = Storage::files_in("uploads");
    /// # }
    /// ```
    pub fn files_in(directory: &str) -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.files_in(directory)
    }

    /// Get all directories
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() {
    /// let dirs = Storage::directories();
    /// # }
    /// ```
    pub fn directories() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read().unwrap();
        manager.directories()
    }

    /// Copy a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::copy("file.txt", "file_copy.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn copy(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.copy(from, to)
    }

    /// Move a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::move_file("old.txt", "new.txt")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn move_file(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.move_file(from, to)
    }

    /// Prepend content to a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::prepend("file.txt", b"Prepended text\n".to_vec())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn prepend(path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write().unwrap();
        manager.prepend(path, data)
    }

    /// Append content to a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::append("file.txt", b"\nAppended text".to_vec())?;
    /// # Ok(())
    /// # }
    /// ```
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
        Storage::put("test_put_get.txt", contents.clone()).unwrap();

        let retrieved = Storage::get("test_put_get.txt").unwrap();
        assert_eq!(retrieved, contents);
    }

    #[test]
    fn test_storage_get_string() {
        let text = "Hello, World!";
        Storage::put("test_string.txt", text.as_bytes().to_vec()).unwrap();

        let retrieved = Storage::get_string("test_string.txt").unwrap();
        assert_eq!(retrieved, text);
    }

    #[test]
    fn test_storage_exists() {
        let path = "test_exists.txt";
        Storage::put(path, b"test".to_vec()).unwrap();

        assert!(Storage::exists(path));
        assert!(!Storage::exists("nonexistent.txt"));
    }

    #[test]
    fn test_storage_delete() {
        let path = "test_delete.txt";
        Storage::put(path, b"test".to_vec()).unwrap();
        assert!(Storage::exists(path));

        Storage::delete(path).unwrap();
        assert!(!Storage::exists(path));
    }

    #[test]
    fn test_storage_size() {
        let contents = b"Hello, World!".to_vec();
        let path = "test_size.txt";
        Storage::put(path, contents.clone()).unwrap();

        let size = Storage::size(path).unwrap();
        assert_eq!(size, contents.len() as u64);
    }

    #[test]
    fn test_storage_files() {
        Storage::put("test_files_1.txt", b"test1".to_vec()).unwrap();
        Storage::put("test_files_2.txt", b"test2".to_vec()).unwrap();

        let files = Storage::files();
        assert!(files.len() >= 2);
    }

    #[test]
    fn test_storage_copy() {
        let from = "test_copy_from.txt";
        let to = "test_copy_to.txt";
        let contents = b"test".to_vec();

        Storage::put(from, contents.clone()).unwrap();
        Storage::copy(from, to).unwrap();

        assert!(Storage::exists(from));
        assert!(Storage::exists(to));
        assert_eq!(Storage::get(from).unwrap(), Storage::get(to).unwrap());
    }

    #[test]
    fn test_storage_move() {
        let from = "test_move_from.txt";
        let to = "test_move_to.txt";
        let contents = b"test".to_vec();

        Storage::put(from, contents.clone()).unwrap();
        Storage::move_file(from, to).unwrap();

        assert!(!Storage::exists(from));
        assert!(Storage::exists(to));
        assert_eq!(Storage::get(to).unwrap(), contents);
    }

    #[test]
    fn test_storage_append() {
        let path = "test_append.txt";
        Storage::put(path, b"Hello".to_vec()).unwrap();
        Storage::append(path, b", World!".to_vec()).unwrap();

        let contents = Storage::get(path).unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[test]
    fn test_storage_prepend() {
        let path = "test_prepend.txt";
        Storage::put(path, b"World!".to_vec()).unwrap();
        Storage::prepend(path, b"Hello, ".to_vec()).unwrap();

        let contents = Storage::get(path).unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[test]
    fn test_storage_disk() {
        let original = Storage::disk_name();

        Storage::disk("s3");
        assert_eq!(Storage::disk_name(), "s3");

        // Restore
        Storage::disk(&original);
    }
}
