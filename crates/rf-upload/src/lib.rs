//! File upload utilities for RustForge
//!
//! This crate provides file upload handling, validation, and image processing.

use axum::extract::Multipart;
use bytes::Bytes;
use mime::Mime;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

/// Upload errors
#[derive(Debug, Error)]
pub enum UploadError {
    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),

    #[error("File too large: {0} bytes (max: {1} bytes)")]
    FileTooLarge(u64, u64),

    #[error("No file provided")]
    NoFile,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Multipart error: {0}")]
    Multipart(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),
}

pub type UploadResult<T> = Result<T, UploadError>;

/// File upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// Allowed MIME types (empty = allow all)
    pub allowed_mime_types: Vec<String>,
    /// Maximum file size in bytes
    pub max_size: Option<u64>,
    /// Storage directory
    pub storage_dir: PathBuf,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            allowed_mime_types: vec![],
            max_size: Some(10 * 1024 * 1024), // 10MB
            storage_dir: PathBuf::from("uploads"),
        }
    }
}

/// Uploaded file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    /// Original filename
    pub filename: String,
    /// Stored path
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// MIME type
    pub mime_type: String,
}

impl UploadedFile {
    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|s| s.to_str())
    }
}

/// File upload handler
pub struct FileUpload {
    filename: String,
    content: Bytes,
    mime_type: Mime,
}

impl FileUpload {
    /// Create from multipart field
    pub async fn from_multipart(multipart: &mut Multipart) -> UploadResult<Self> {
        let field = multipart
            .next_field()
            .await
            .map_err(|e| UploadError::Multipart(e.to_string()))?
            .ok_or(UploadError::NoFile)?;

        let filename = field
            .file_name()
            .ok_or(UploadError::NoFile)?
            .to_string();

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream");

        let mime_type: Mime = content_type
            .parse()
            .unwrap_or(mime::APPLICATION_OCTET_STREAM);

        let content = field
            .bytes()
            .await
            .map_err(|e| UploadError::Multipart(e.to_string()))?;

        Ok(Self {
            filename,
            content,
            mime_type,
        })
    }

    /// Validate MIME type
    pub fn validate_mime_type(self, allowed: &[&str]) -> UploadResult<Self> {
        if allowed.is_empty() {
            return Ok(self);
        }

        let mime_str = self.mime_type.to_string();
        if allowed.iter().any(|&a| mime_str.starts_with(a)) {
            Ok(self)
        } else {
            Err(UploadError::InvalidMimeType(mime_str))
        }
    }

    /// Validate file size
    pub fn validate_max_size(self, max_bytes: u64) -> UploadResult<Self> {
        let size = self.content.len() as u64;
        if size > max_bytes {
            Err(UploadError::FileTooLarge(size, max_bytes))
        } else {
            Ok(self)
        }
    }

    /// Get file size
    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }

    /// Get MIME type
    pub fn mime_type(&self) -> &Mime {
        &self.mime_type
    }

    /// Get filename
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// Store file to disk
    pub async fn store<P: AsRef<Path>>(self, directory: P) -> UploadResult<UploadedFile> {
        let dir = directory.as_ref();
        tokio::fs::create_dir_all(dir).await?;

        // Generate unique filename
        let filename = sanitize_filename(&self.filename);
        let path = dir.join(&filename);

        // Write file
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(&self.content).await?;
        file.flush().await?;

        Ok(UploadedFile {
            filename,
            path,
            size: self.content.len() as u64,
            mime_type: self.mime_type.to_string(),
        })
    }

    /// Store with custom filename
    pub async fn store_as<P: AsRef<Path>>(
        self,
        directory: P,
        filename: &str,
    ) -> UploadResult<UploadedFile> {
        let dir = directory.as_ref();
        tokio::fs::create_dir_all(dir).await?;

        let filename = sanitize_filename(filename);
        let path = dir.join(&filename);

        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(&self.content).await?;
        file.flush().await?;

        Ok(UploadedFile {
            filename,
            path,
            size: self.content.len() as u64,
            mime_type: self.mime_type.to_string(),
        })
    }
}

/// Sanitize filename for security
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Image processing (requires "image-processing" feature)
#[cfg(feature = "image-processing")]
pub mod image_processing {
    use super::*;
    use image::{DynamicImage, ImageFormat};

    /// Image resize mode
    #[derive(Debug, Clone, Copy)]
    pub enum ResizeMode {
        /// Fit within dimensions (preserves aspect ratio)
        Fit,
        /// Fill dimensions (may crop)
        Fill,
        /// Exact dimensions (may distort)
        Exact,
    }

    /// Image processor
    pub struct ImageProcessor {
        image: DynamicImage,
    }

    impl ImageProcessor {
        /// Load from file
        pub fn from_path<P: AsRef<Path>>(path: P) -> UploadResult<Self> {
            let image = image::open(path)
                .map_err(|e| UploadError::ImageProcessing(e.to_string()))?;
            Ok(Self { image })
        }

        /// Load from bytes
        pub fn from_bytes(bytes: &[u8]) -> UploadResult<Self> {
            let image = image::load_from_memory(bytes)
                .map_err(|e| UploadError::ImageProcessing(e.to_string()))?;
            Ok(Self { image })
        }

        /// Resize image
        pub fn resize(mut self, width: u32, height: u32, mode: ResizeMode) -> Self {
            self.image = match mode {
                ResizeMode::Fit => self.image.resize(
                    width,
                    height,
                    image::imageops::FilterType::Lanczos3,
                ),
                ResizeMode::Fill => self.image.resize_to_fill(
                    width,
                    height,
                    image::imageops::FilterType::Lanczos3,
                ),
                ResizeMode::Exact => self.image.resize_exact(
                    width,
                    height,
                    image::imageops::FilterType::Lanczos3,
                ),
            };
            self
        }

        /// Crop image
        pub fn crop(mut self, x: u32, y: u32, width: u32, height: u32) -> Self {
            self.image = self.image.crop_imm(x, y, width, height);
            self
        }

        /// Save to file
        pub fn save<P: AsRef<Path>>(self, path: P) -> UploadResult<()> {
            self.image
                .save(path)
                .map_err(|e| UploadError::ImageProcessing(e.to_string()))
        }

        /// Convert to bytes
        pub fn to_bytes(self, format: ImageFormat) -> UploadResult<Vec<u8>> {
            let mut bytes = Vec::new();
            self.image
                .write_to(&mut std::io::Cursor::new(&mut bytes), format)
                .map_err(|e| UploadError::ImageProcessing(e.to_string()))?;
            Ok(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test.jpg"), "test.jpg");
        assert_eq!(sanitize_filename("test file.jpg"), "test_file.jpg");
        assert_eq!(sanitize_filename("../../../etc/passwd"), ".._.._.._etc_passwd");
        assert_eq!(sanitize_filename("file with spaces.png"), "file_with_spaces.png");
    }

    #[test]
    fn test_upload_config_default() {
        let config = UploadConfig::default();
        assert_eq!(config.max_size, Some(10 * 1024 * 1024));
        assert_eq!(config.storage_dir, PathBuf::from("uploads"));
    }

    #[test]
    fn test_uploaded_file_extension() {
        let file = UploadedFile {
            filename: "test.jpg".to_string(),
            path: PathBuf::from("uploads/test.jpg"),
            size: 1024,
            mime_type: "image/jpeg".to_string(),
        };

        assert_eq!(file.extension(), Some("jpg"));
    }

    #[tokio::test]
    async fn test_file_upload_validate_size() {
        let upload = FileUpload {
            filename: "test.txt".to_string(),
            content: Bytes::from(vec![0u8; 1000]),
            mime_type: mime::TEXT_PLAIN,
        };

        // Should pass
        let result = upload.clone().validate_max_size(2000);
        assert!(result.is_ok());

        // Should fail
        let result = upload.validate_max_size(500);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_upload_validate_mime_type() {
        let upload = FileUpload {
            filename: "test.jpg".to_string(),
            content: Bytes::from(vec![]),
            mime_type: mime::IMAGE_JPEG,
        };

        // Should pass
        let result = upload.clone().validate_mime_type(&["image/"]);
        assert!(result.is_ok());

        // Should fail
        let result = upload.validate_mime_type(&["video/"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_upload_size() {
        let upload = FileUpload {
            filename: "test.txt".to_string(),
            content: Bytes::from(vec![0u8; 1234]),
            mime_type: mime::TEXT_PLAIN,
        };

        assert_eq!(upload.size(), 1234);
    }

    #[test]
    fn test_file_upload_filename() {
        let upload = FileUpload {
            filename: "test.txt".to_string(),
            content: Bytes::from(vec![]),
            mime_type: mime::TEXT_PLAIN,
        };

        assert_eq!(upload.filename(), "test.txt");
    }

    #[test]
    fn test_file_upload_mime_type() {
        let upload = FileUpload {
            filename: "test.txt".to_string(),
            content: Bytes::from(vec![]),
            mime_type: mime::TEXT_PLAIN,
        };

        assert_eq!(*upload.mime_type(), mime::TEXT_PLAIN);
    }

    #[tokio::test]
    async fn test_store_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let upload = FileUpload {
            filename: "test.txt".to_string(),
            content: Bytes::from("Hello, World!"),
            mime_type: mime::TEXT_PLAIN,
        };

        let result = upload.store(temp_dir.path()).await;
        assert!(result.is_ok());

        let uploaded = result.unwrap();
        assert!(uploaded.path.exists());
        assert_eq!(uploaded.size, 13);
    }
}
