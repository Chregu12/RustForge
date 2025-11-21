//! Media library for file and image management

use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use image::{ImageFormat, imageops::FilterType};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{CmsError, CmsResult};

/// Media file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    /// Unique file ID
    pub id: String,

    /// Original filename
    pub filename: String,

    /// File path (relative to storage root)
    pub path: String,

    /// MIME type
    pub mime_type: String,

    /// File size in bytes
    pub size: u64,

    /// SHA256 hash
    pub hash: String,

    /// Image dimensions (if applicable)
    pub dimensions: Option<(u32, u32)>,

    /// Upload timestamp
    pub uploaded_at: DateTime<Utc>,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a file
    async fn store(&self, path: &str, data: &[u8]) -> CmsResult<()>;

    /// Retrieve a file
    async fn retrieve(&self, path: &str) -> CmsResult<Vec<u8>>;

    /// Delete a file
    async fn delete(&self, path: &str) -> CmsResult<()>;

    /// Check if file exists
    async fn exists(&self, path: &str) -> CmsResult<bool>;

    /// Get file URL
    fn url(&self, path: &str) -> String;
}

/// Local filesystem storage
pub struct LocalStorage {
    base_path: PathBuf,
    base_url: String,
}

impl LocalStorage {
    pub fn new<P: AsRef<Path>>(base_path: P, base_url: impl Into<String>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, path: &str, data: &[u8]) -> CmsResult<()> {
        let full_path = self.base_path.join(path);

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write file
        let mut file = fs::File::create(&full_path).await?;
        file.write_all(data).await?;

        Ok(())
    }

    async fn retrieve(&self, path: &str) -> CmsResult<Vec<u8>> {
        let full_path = self.base_path.join(path);
        let data = fs::read(&full_path).await?;
        Ok(data)
    }

    async fn delete(&self, path: &str) -> CmsResult<()> {
        let full_path = self.base_path.join(path);
        fs::remove_file(&full_path).await?;
        Ok(())
    }

    async fn exists(&self, path: &str) -> CmsResult<bool> {
        let full_path = self.base_path.join(path);
        Ok(full_path.exists())
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), path)
    }
}

/// Media library manager
pub struct MediaLibrary {
    storage: Arc<dyn StorageBackend>,
    /// ID -> Path mapping (simulates database)
    file_paths: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
}

impl MediaLibrary {
    /// Create a new media library with local storage
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Self {
        let storage = LocalStorage::new(storage_path, "/media");
        Self {
            storage: Arc::new(storage),
            file_paths: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Create with custom storage backend
    pub fn with_storage(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            storage,
            file_paths: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Upload a file
    pub async fn upload(
        &self,
        filename: &str,
        data: Vec<u8>,
    ) -> CmsResult<MediaFile> {
        let id = Uuid::new_v4().to_string();

        // Calculate hash
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = hex::encode(hasher.finalize());

        // Detect MIME type
        let mime_type = mime_guess::from_path(filename)
            .first_or_octet_stream()
            .to_string();

        // Determine storage path
        let extension = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let path = format!("{}/{}.{}", &id[..2], id, extension);

        // Process image if applicable
        let dimensions = if mime_type.starts_with("image/") {
            self.get_image_dimensions(&data).ok()
        } else {
            None
        };

        // Store file
        self.storage.store(&path, &data).await?;

        // Store path mapping
        {
            let mut paths = self.file_paths.write().await;
            paths.insert(id.clone(), path.clone());
        }

        Ok(MediaFile {
            id,
            filename: filename.to_string(),
            path: path.clone(),
            mime_type,
            size: data.len() as u64,
            hash,
            dimensions,
            uploaded_at: Utc::now(),
            metadata: serde_json::json!({}),
        })
    }

    /// Get file by ID
    pub async fn get(&self, file_id: &str) -> CmsResult<Vec<u8>> {
        // Find file by ID (simplified - in production, use database lookup)
        let path = self.resolve_path(file_id).await?;
        self.storage.retrieve(&path).await
    }

    /// Delete file
    pub async fn delete(&self, file_id: &str) -> CmsResult<()> {
        let path = self.resolve_path(file_id).await?;
        self.storage.delete(&path).await
    }

    /// Get file URL
    pub async fn url(&self, file_id: &str) -> String {
        if let Ok(path) = self.resolve_path(file_id).await {
            self.storage.url(&path)
        } else {
            String::new()
        }
    }

    /// Generate thumbnail
    pub async fn thumbnail(
        &self,
        file_id: &str,
        width: u32,
        height: u32,
    ) -> CmsResult<MediaFile> {
        // Get original image
        let data = self.get(file_id).await?;

        // Load image
        let img = image::load_from_memory(&data)
            .map_err(|e| CmsError::ImageError(e.to_string()))?;

        // Resize
        let thumb = img.resize(width, height, FilterType::Lanczos3);

        // Encode as JPEG
        let mut buffer = Vec::new();
        thumb.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            ImageFormat::Jpeg,
        ).map_err(|e| CmsError::ImageError(e.to_string()))?;

        // Upload thumbnail
        let thumb_filename = format!("thumb_{}x{}_{}.jpg", width, height, file_id);
        self.upload(&thumb_filename, buffer).await
    }

    /// Crop image
    pub async fn crop(
        &self,
        file_id: &str,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> CmsResult<MediaFile> {
        let data = self.get(file_id).await?;

        let img = image::load_from_memory(&data)
            .map_err(|e| CmsError::ImageError(e.to_string()))?;

        let cropped = img.crop_imm(x, y, width, height);

        let mut buffer = Vec::new();
        cropped.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            ImageFormat::Jpeg,
        ).map_err(|e| CmsError::ImageError(e.to_string()))?;

        let crop_filename = format!("crop_{}x{}_{}x{}_{}.jpg", x, y, width, height, file_id);
        self.upload(&crop_filename, buffer).await
    }

    /// Get image dimensions
    fn get_image_dimensions(&self, data: &[u8]) -> CmsResult<(u32, u32)> {
        let img = image::load_from_memory(data)
            .map_err(|e| CmsError::ImageError(e.to_string()))?;

        Ok((img.width(), img.height()))
    }

    /// Resolve file path from ID (simplified)
    async fn resolve_path(&self, file_id: &str) -> CmsResult<String> {
        // In production, this would query a database
        // For now, we use our in-memory mapping
        let paths = self.file_paths.read().await;

        paths
            .get(file_id)
            .cloned()
            .ok_or_else(|| CmsError::MediaError(format!("File not found: {}", file_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use image::DynamicImage;

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path(), "/media");

        // Store file
        let data = b"Hello, World!";
        storage.store("test.txt", data).await.unwrap();

        // Retrieve file
        let retrieved = storage.retrieve("test.txt").await.unwrap();
        assert_eq!(retrieved, data);

        // Check exists
        assert!(storage.exists("test.txt").await.unwrap());

        // Delete file
        storage.delete("test.txt").await.unwrap();
        assert!(!storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_media_upload() {
        let temp_dir = TempDir::new().unwrap();
        let media = MediaLibrary::new(temp_dir.path());

        let data = b"Test file content".to_vec();
        let file = media.upload("test.txt", data).await.unwrap();

        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.size, 17);
        assert!(!file.id.is_empty());
        assert!(!file.hash.is_empty());
    }

    #[tokio::test]
    async fn test_image_upload_with_dimensions() {
        let temp_dir = TempDir::new().unwrap();
        let media = MediaLibrary::new(temp_dir.path());

        // Create a small test image (1x1 red pixel)
        let img = DynamicImage::ImageRgb8(
            image::RgbImage::from_pixel(1, 1, image::Rgb([255, 0, 0]))
        );

        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            ImageFormat::Png,
        ).unwrap();

        let file = media.upload("test.png", buffer).await.unwrap();

        assert_eq!(file.filename, "test.png");
        assert!(file.mime_type.starts_with("image/"));
        assert_eq!(file.dimensions, Some((1, 1)));
    }

    #[tokio::test]
    async fn test_thumbnail_generation() {
        let temp_dir = TempDir::new().unwrap();
        let media = MediaLibrary::new(temp_dir.path());

        // Create a 100x100 test image
        let img = DynamicImage::ImageRgb8(
            image::RgbImage::from_pixel(100, 100, image::Rgb([0, 255, 0]))
        );

        let mut buffer = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut buffer),
            ImageFormat::Png,
        ).unwrap();

        let file = media.upload("original.png", buffer).await.unwrap();

        // Generate thumbnail
        let thumb = media.thumbnail(&file.id, 50, 50).await.unwrap();

        assert!(thumb.filename.contains("thumb"));
        assert_eq!(thumb.dimensions, Some((50, 50)));
    }

    #[test]
    fn test_hash_generation() {
        let data = b"Test data";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize());

        assert_eq!(hash.len(), 64); // SHA256 is 64 hex characters
    }

    #[test]
    fn test_mime_type_detection() {
        let mime = mime_guess::from_path("test.jpg")
            .first_or_octet_stream()
            .to_string();

        assert_eq!(mime, "image/jpeg");

        let mime = mime_guess::from_path("test.pdf")
            .first_or_octet_stream()
            .to_string();

        assert_eq!(mime, "application/pdf");
    }
}
