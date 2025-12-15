//! Storage fake implementation for testing
//!
//! Provides a fake Storage implementation that records all file operations
//! and allows assertions on what was stored, deleted, or accessed.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// File visibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Public file (accessible via URL)
    Public,
    /// Private file (not accessible via URL)
    Private,
}

/// Record of a stored file
#[derive(Debug, Clone)]
pub struct FileRecord {
    /// File path
    pub path: String,
    /// File contents
    pub content: Vec<u8>,
    /// Disk name where file is stored
    pub disk: String,
    /// File visibility
    pub visibility: Visibility,
    /// File MIME type
    pub mime_type: Option<String>,
    /// File size in bytes
    pub size: u64,
}

/// Storage fake for testing
///
/// Records all file operations and provides assertion methods to verify behavior.
/// Supports multiple disks like Laravel's Storage facade.
///
/// # Example
///
/// ```
/// use rf_testing::fakes::StorageFake;
///
/// let fake = StorageFake::new();
///
/// // Store files
/// fake.disk("local").put("test.txt", b"Hello, World!".to_vec());
///
/// // Assert
/// fake.assert_exists("test.txt");
/// fake.assert_missing("missing.txt");
/// ```
#[derive(Clone)]
pub struct StorageFake {
    files: Arc<Mutex<Vec<FileRecord>>>,
    current_disk: Arc<Mutex<String>>,
}

impl StorageFake {
    /// Create a new storage fake with default disk
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(Vec::new())),
            current_disk: Arc::new(Mutex::new("default".to_string())),
        }
    }

    /// Select a disk to operate on (like Laravel Storage::disk())
    pub fn disk(&self, name: &str) -> Self {
        let mut current = self.current_disk.lock().unwrap();
        *current = name.to_string();
        self.clone()
    }

    /// Get the current disk name
    pub fn current_disk(&self) -> String {
        self.current_disk.lock().unwrap().clone()
    }

    /// Store a file
    pub fn put(&self, path: &str, content: Vec<u8>) -> &Self {
        self.put_with_visibility(path, content, Visibility::Public)
    }

    /// Store a file with specific visibility
    pub fn put_with_visibility(
        &self,
        path: &str,
        content: Vec<u8>,
        visibility: Visibility,
    ) -> &Self {
        let disk = self.current_disk.lock().unwrap().clone();
        let size = content.len() as u64;
        let mime_type = detect_mime_type(path);

        let record = FileRecord {
            path: path.to_string(),
            content,
            disk: disk.clone(),
            visibility,
            mime_type,
            size,
        };

        let mut files = self.files.lock().unwrap();
        files.retain(|f| !(f.path == path && f.disk == disk));
        files.push(record);

        self
    }

    /// Get file contents
    pub fn get(&self, path: &str) -> Option<Vec<u8>> {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();
        files
            .iter()
            .find(|f| f.path == path && f.disk == disk)
            .map(|f| f.content.clone())
    }

    /// Check if file exists
    pub fn exists(&self, path: &str) -> bool {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();
        files.iter().any(|f| f.path == path && f.disk == disk)
    }

    /// Check if file exists on any disk
    pub fn exists_on_any_disk(&self, path: &str) -> bool {
        let files = self.files.lock().unwrap();
        files.iter().any(|f| f.path == path)
    }

    /// Delete a file
    pub fn delete(&self, path: &str) -> &Self {
        let disk = self.current_disk.lock().unwrap().clone();
        let mut files = self.files.lock().unwrap();
        files.retain(|f| !(f.path == path && f.disk == disk));
        self
    }

    /// Copy a file from one location to another
    pub fn copy(&self, from: &str, to: &str) -> &Self {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();

        if let Some(source) = files.iter().find(|f| f.path == from && f.disk == disk) {
            let new_record = source.clone();
            drop(files);
            self.put_with_visibility(to, new_record.content, new_record.visibility);
        }

        self
    }

    /// Move a file from one location to another
    pub fn move_file(&self, from: &str, to: &str) -> &Self {
        self.copy(from, to);
        self.delete(from);
        self
    }

    /// Get all files in a directory
    pub fn files(&self, directory: &str) -> Vec<String> {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();

        let prefix = if directory.is_empty() {
            "".to_string()
        } else {
            format!("{}/", directory.trim_end_matches('/'))
        };

        files
            .iter()
            .filter(|f| f.disk == disk && f.path.starts_with(&prefix))
            .map(|f| f.path.clone())
            .collect()
    }

    /// Get all directories within a path
    pub fn directories(&self, path: &str) -> Vec<String> {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();

        let prefix = if path.is_empty() {
            "".to_string()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let mut dirs = HashSet::new();

        for file in files.iter() {
            if file.disk == disk && file.path.starts_with(&prefix) {
                let remainder = &file.path[prefix.len()..];
                if let Some(slash_pos) = remainder.find('/') {
                    let dir = &remainder[..slash_pos];
                    dirs.insert(format!("{}{}", prefix, dir));
                }
            }
        }

        dirs.into_iter().collect()
    }

    /// Get all stored files across all disks
    pub fn all_files(&self) -> Vec<FileRecord> {
        self.files.lock().unwrap().clone()
    }

    /// Get all files on the current disk
    pub fn all_files_on_disk(&self) -> Vec<FileRecord> {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();
        files.iter().filter(|f| f.disk == disk).cloned().collect()
    }

    /// Clear all stored files
    pub fn clear(&self) {
        self.files.lock().unwrap().clear();
    }

    /// Get the number of stored files on current disk
    pub fn count(&self) -> usize {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();
        files.iter().filter(|f| f.disk == disk).count()
    }

    /// Get the total number of stored files across all disks
    pub fn total_count(&self) -> usize {
        self.files.lock().unwrap().len()
    }

    /// Get file size
    pub fn size(&self, path: &str) -> Option<u64> {
        let disk = self.current_disk.lock().unwrap().clone();
        let files = self.files.lock().unwrap();
        files
            .iter()
            .find(|f| f.path == path && f.disk == disk)
            .map(|f| f.size)
    }

    /// Assert that a file exists
    pub fn assert_exists(&self, path: &str) {
        let disk = self.current_disk.lock().unwrap().clone();

        if !self.exists(path) {
            let files = self.files.lock().unwrap();
            let disk_files: Vec<&str> = files
                .iter()
                .filter(|f| f.disk == disk)
                .map(|f| f.path.as_str())
                .collect();

            panic!(
                "Failed asserting that file '{}' exists on disk '{}'. Files on disk: {:?}",
                path, disk, disk_files
            );
        }
    }

    /// Assert that a file is missing
    pub fn assert_missing(&self, path: &str) {
        let disk = self.current_disk.lock().unwrap().clone();

        if self.exists(path) {
            panic!(
                "Failed asserting that file '{}' is missing on disk '{}'",
                path, disk
            );
        }
    }

    /// Assert that a file has specific content
    pub fn assert_content(&self, path: &str, expected_content: &[u8]) {
        let disk = self.current_disk.lock().unwrap().clone();

        match self.get(path) {
            Some(content) => {
                if content != expected_content {
                    panic!(
                        "Failed asserting that file '{}' on disk '{}' has expected content",
                        path, disk
                    );
                }
            }
            None => {
                panic!(
                    "Failed asserting content - file '{}' does not exist on disk '{}'",
                    path, disk
                );
            }
        }
    }

    /// Assert that a specific number of files exist on the current disk
    pub fn assert_count(&self, expected: usize) {
        let disk = self.current_disk.lock().unwrap().clone();
        let count = self.count();

        if count != expected {
            panic!(
                "Failed asserting that disk '{}' has {} files. Actually has {} files.",
                disk, expected, count
            );
        }
    }

    /// Assert that no files exist on the current disk
    pub fn assert_empty(&self) {
        self.assert_count(0);
    }
}

impl Default for StorageFake {
    fn default() -> Self {
        Self::new()
    }
}

/// Fake uploaded file for testing
#[derive(Debug, Clone)]
pub struct FakeUploadedFile {
    /// Original filename
    pub name: String,
    /// File content
    pub content: Vec<u8>,
    /// MIME type
    pub mime_type: String,
    /// File size in bytes
    pub size: u64,
}

impl FakeUploadedFile {
    /// Create a new fake uploaded file
    pub fn new(name: impl Into<String>, content: Vec<u8>, mime_type: impl Into<String>) -> Self {
        let name = name.into();
        let size = content.len() as u64;
        let mime_type = mime_type.into();
        Self {
            name,
            content,
            mime_type,
            size,
        }
    }

    /// Create a fake text file
    pub fn text(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(name, content.into().into_bytes(), "text/plain")
    }

    /// Create a fake JSON file
    pub fn json(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(name, content.into().into_bytes(), "application/json")
    }

    /// Get file extension
    pub fn extension(&self) -> Option<&str> {
        self.name.rsplit('.').next()
    }
}

/// Create a fake file with random content
pub fn create_fake_file(name: impl Into<String>, size_bytes: usize) -> FakeUploadedFile {
    let name = name.into();
    let content = vec![b'x'; size_bytes];
    let mime_type =
        detect_mime_type(&name).unwrap_or_else(|| "application/octet-stream".to_string());
    FakeUploadedFile::new(name, content, mime_type)
}

/// Create a fake image file with specific dimensions
pub fn create_fake_image(name: impl Into<String>, width: u32, height: u32) -> FakeUploadedFile {
    let name = name.into();
    let content = format!("FAKE_IMAGE:{}x{}", width, height).into_bytes();

    let mime_type = if name.ends_with(".png") {
        "image/png"
    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        "image/jpeg"
    } else if name.ends_with(".gif") {
        "image/gif"
    } else if name.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    };

    FakeUploadedFile::new(name, content, mime_type)
}

/// Detect MIME type from filename extension
fn detect_mime_type(filename: &str) -> Option<String> {
    let extension = filename.rsplit('.').next()?;

    let mime = match extension.to_lowercase().as_str() {
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => return None,
    };

    Some(mime.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_fake_creation() {
        let fake = StorageFake::new();
        assert_eq!(fake.count(), 0);
        assert_eq!(fake.current_disk(), "default");
    }

    #[test]
    fn test_put_and_get() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"Hello, World!".to_vec());
        assert_eq!(fake.get("test.txt"), Some(b"Hello, World!".to_vec()));
    }

    #[test]
    fn test_exists() {
        let fake = StorageFake::new();
        assert!(!fake.exists("test.txt"));
        fake.put("test.txt", b"content".to_vec());
        assert!(fake.exists("test.txt"));
    }

    #[test]
    fn test_delete() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"content".to_vec());
        assert!(fake.exists("test.txt"));
        fake.delete("test.txt");
        assert!(!fake.exists("test.txt"));
    }

    #[test]
    fn test_copy() {
        let fake = StorageFake::new();
        fake.put("original.txt", b"content".to_vec());
        fake.copy("original.txt", "copy.txt");
        assert!(fake.exists("original.txt"));
        assert!(fake.exists("copy.txt"));
        assert_eq!(fake.get("copy.txt"), Some(b"content".to_vec()));
    }

    #[test]
    fn test_move_file() {
        let fake = StorageFake::new();
        fake.put("old.txt", b"content".to_vec());
        fake.move_file("old.txt", "new.txt");
        assert!(!fake.exists("old.txt"));
        assert!(fake.exists("new.txt"));
    }

    #[test]
    fn test_disk_selection() {
        let fake = StorageFake::new();
        fake.disk("local").put("file1.txt", b"local content".to_vec());
        fake.disk("s3").put("file1.txt", b"s3 content".to_vec());

        assert_eq!(
            fake.disk("local").get("file1.txt"),
            Some(b"local content".to_vec())
        );
        assert_eq!(
            fake.disk("s3").get("file1.txt"),
            Some(b"s3 content".to_vec())
        );
    }

    #[test]
    fn test_files_in_directory() {
        let fake = StorageFake::new();
        fake.put("dir1/file1.txt", b"1".to_vec());
        fake.put("dir1/file2.txt", b"2".to_vec());
        fake.put("dir2/file3.txt", b"3".to_vec());

        let files = fake.files("dir1");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_size() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"Hello".to_vec());
        assert_eq!(fake.size("test.txt"), Some(5));
    }

    #[test]
    fn test_assert_exists() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"content".to_vec());
        fake.assert_exists("test.txt");
    }

    #[test]
    #[should_panic(expected = "Failed asserting that file 'missing.txt' exists")]
    fn test_assert_exists_fails() {
        let fake = StorageFake::new();
        fake.assert_exists("missing.txt");
    }

    #[test]
    fn test_assert_missing() {
        let fake = StorageFake::new();
        fake.assert_missing("test.txt");
    }

    #[test]
    #[should_panic(expected = "Failed asserting that file 'test.txt' is missing")]
    fn test_assert_missing_fails() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"content".to_vec());
        fake.assert_missing("test.txt");
    }

    #[test]
    fn test_assert_content() {
        let fake = StorageFake::new();
        fake.put("test.txt", b"Hello, World!".to_vec());
        fake.assert_content("test.txt", b"Hello, World!");
    }

    #[test]
    fn test_assert_count() {
        let fake = StorageFake::new();
        fake.put("file1.txt", b"1".to_vec());
        fake.put("file2.txt", b"2".to_vec());
        fake.assert_count(2);
    }

    #[test]
    fn test_assert_empty() {
        let fake = StorageFake::new();
        fake.assert_empty();
    }

    #[test]
    fn test_clear() {
        let fake = StorageFake::new();
        fake.put("file1.txt", b"1".to_vec());
        fake.put("file2.txt", b"2".to_vec());
        assert_eq!(fake.count(), 2);
        fake.clear();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_fake_uploaded_file() {
        let file = FakeUploadedFile::new("test.txt", b"Hello".to_vec(), "text/plain");
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.content, b"Hello");
        assert_eq!(file.mime_type, "text/plain");
        assert_eq!(file.size, 5);
        assert_eq!(file.extension(), Some("txt"));
    }

    #[test]
    fn test_fake_uploaded_file_text() {
        let file = FakeUploadedFile::text("note.txt", "Hello, World!");
        assert_eq!(file.mime_type, "text/plain");
        assert_eq!(file.content, b"Hello, World!");
    }

    #[test]
    fn test_create_fake_file() {
        let file = create_fake_file("test.txt", 1024);
        assert_eq!(file.name, "test.txt");
        assert_eq!(file.size, 1024);
        assert_eq!(file.content.len(), 1024);
    }

    #[test]
    fn test_create_fake_image() {
        let image = create_fake_image("avatar.png", 800, 600);
        assert_eq!(image.name, "avatar.png");
        assert_eq!(image.mime_type, "image/png");
    }
}
