//! Storage facade providing Laravel-style static storage API

use crate::manager::GLOBAL_STORAGE;

/// The Storage facade providing a static-like API for file storage.
///
/// This is the main entry point for file storage operations in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_storage_facade::Storage;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Put a file
/// Storage::put("file.txt", b"Hello, World!".to_vec()).await?;
///
/// // Get a file
/// let contents = Storage::get("file.txt").await?;
/// println!("File contents: {:?}", String::from_utf8_lossy(&contents));
///
/// // Check if exists
/// if Storage::exists("file.txt").await {
///     println!("File exists!");
/// }
///
/// // Get file size
/// let size = Storage::size("file.txt").await?;
/// println!("File size: {} bytes", size);
///
/// // Delete file
/// Storage::delete("file.txt").await?;
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
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::put("file.txt", b"Hello, World!".to_vec()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put(path: &str, contents: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.put(path, contents)
    }

    /// Get a file's contents
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let contents = Storage::get("file.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(path: &str) -> Result<Vec<u8>, String> {
        let manager = GLOBAL_STORAGE.read();
        manager.get(path)
    }

    /// Get a file's contents as a string
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let text = Storage::get_string("file.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_string(path: &str) -> Result<String, String> {
        let contents = Self::get(path).await?;
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
    /// # async fn example() {
    /// if Storage::exists("file.txt").await {
    ///     println!("File exists!");
    /// }
    /// # }
    /// ```
    pub async fn exists(path: &str) -> bool {
        let manager = GLOBAL_STORAGE.read();
        manager.exists(path)
    }

    /// Delete a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::delete("file.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(path: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.delete(path)
    }

    /// Get the size of a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let size = Storage::size("file.txt").await?;
    /// println!("File size: {} bytes", size);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn size(path: &str) -> Result<u64, String> {
        let manager = GLOBAL_STORAGE.read();
        manager.size(path)
    }

    /// List all files
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() {
    /// let files = Storage::files().await;
    /// for file in files {
    ///     println!("File: {}", file);
    /// }
    /// # }
    /// ```
    pub async fn files() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read();
        manager.files()
    }

    /// List all files in a directory
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() {
    /// let files = Storage::files_in("uploads").await;
    /// # }
    /// ```
    pub async fn files_in(directory: &str) -> Vec<String> {
        let manager = GLOBAL_STORAGE.read();
        manager.files_in(directory)
    }

    /// Get all directories
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() {
    /// let dirs = Storage::directories().await;
    /// # }
    /// ```
    pub async fn directories() -> Vec<String> {
        let manager = GLOBAL_STORAGE.read();
        manager.directories()
    }

    /// Copy a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::copy("file.txt", "file_copy.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn copy(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.copy(from, to)
    }

    /// Move a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::move_file("old.txt", "new.txt").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn move_file(from: &str, to: &str) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.move_file(from, to)
    }

    /// Prepend content to a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::prepend("file.txt", b"Prepended text\n".to_vec()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn prepend(path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.prepend(path, data)
    }

    /// Append content to a file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_storage_facade::Storage;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Storage::append("file.txt", b"\nAppended text".to_vec()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn append(path: &str, data: Vec<u8>) -> Result<(), String> {
        let mut manager = GLOBAL_STORAGE.write();
        manager.append(path, data)
    }

    /// Get the current disk name
    pub async fn disk_name() -> String {
        let manager = GLOBAL_STORAGE.read();
        manager.disk_name().to_string()
    }

    /// Set the disk to use
    pub async fn disk(name: &str) {
        let mut manager = GLOBAL_STORAGE.write();
        manager.set_disk(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_put_get() {
        let contents = b"Hello, World!".to_vec();
        Storage::put("test_put_get.txt", contents.clone()).await.unwrap();

        let retrieved = Storage::get("test_put_get.txt").await.unwrap();
        assert_eq!(retrieved, contents);
    }

    #[tokio::test]
    async fn test_storage_get_string() {
        let text = "Hello, World!";
        Storage::put("test_string.txt", text.as_bytes().to_vec()).await.unwrap();

        let retrieved = Storage::get_string("test_string.txt").await.unwrap();
        assert_eq!(retrieved, text);
    }

    #[tokio::test]
    async fn test_storage_exists() {
        let path = "test_exists.txt";
        Storage::put(path, b"test".to_vec()).await.unwrap();

        assert!(Storage::exists(path).await);
        assert!(!Storage::exists("nonexistent.txt").await);
    }

    #[tokio::test]
    async fn test_storage_delete() {
        let path = "test_delete.txt";
        Storage::put(path, b"test".to_vec()).await.unwrap();
        assert!(Storage::exists(path).await);

        Storage::delete(path).await.unwrap();
        assert!(!Storage::exists(path).await);
    }

    #[tokio::test]
    async fn test_storage_size() {
        let contents = b"Hello, World!".to_vec();
        let path = "test_size.txt";
        Storage::put(path, contents.clone()).await.unwrap();

        let size = Storage::size(path).await.unwrap();
        assert_eq!(size, contents.len() as u64);
    }

    #[tokio::test]
    async fn test_storage_files() {
        Storage::put("test_files_1.txt", b"test1".to_vec()).await.unwrap();
        Storage::put("test_files_2.txt", b"test2".to_vec()).await.unwrap();

        let files = Storage::files().await;
        assert!(files.len() >= 2);
    }

    #[tokio::test]
    async fn test_storage_copy() {
        let from = "test_copy_from.txt";
        let to = "test_copy_to.txt";
        let contents = b"test".to_vec();

        Storage::put(from, contents.clone()).await.unwrap();
        Storage::copy(from, to).await.unwrap();

        assert!(Storage::exists(from).await);
        assert!(Storage::exists(to).await);
        assert_eq!(Storage::get(from).await.unwrap(), Storage::get(to).await.unwrap());
    }

    #[tokio::test]
    async fn test_storage_move() {
        let from = "test_move_from.txt";
        let to = "test_move_to.txt";
        let contents = b"test".to_vec();

        Storage::put(from, contents.clone()).await.unwrap();
        Storage::move_file(from, to).await.unwrap();

        assert!(!Storage::exists(from).await);
        assert!(Storage::exists(to).await);
        assert_eq!(Storage::get(to).await.unwrap(), contents);
    }

    #[tokio::test]
    async fn test_storage_append() {
        let path = "test_append.txt";
        Storage::put(path, b"Hello".to_vec()).await.unwrap();
        Storage::append(path, b", World!".to_vec()).await.unwrap();

        let contents = Storage::get(path).await.unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_storage_prepend() {
        let path = "test_prepend.txt";
        Storage::put(path, b"World!".to_vec()).await.unwrap();
        Storage::prepend(path, b"Hello, ".to_vec()).await.unwrap();

        let contents = Storage::get(path).await.unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_storage_disk() {
        let original = Storage::disk_name().await;

        Storage::disk("s3").await;
        assert_eq!(Storage::disk_name().await, "s3");

        // Restore
        Storage::disk(&original).await;
    }
}
