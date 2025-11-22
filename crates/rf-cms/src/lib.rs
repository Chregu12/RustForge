//! # rf-cms - Content Management System
//!
//! Complete CMS features for RustForge including media management, WYSIWYG editors,
//! and content versioning.
//!
//! ## Features
//!
//! - **Media Library**: Upload, storage, and image processing
//! - **WYSIWYG Integration**: TinyMCE/CKEditor helpers
//! - **Content Revisions**: Version tracking and rollback
//! - **Metadata Extraction**: Automatic file metadata
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_cms::MediaLibrary;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let media = MediaLibrary::new("storage/media");
//!
//! // Upload an image
//! let image_bytes = vec![0u8; 100]; // Example image data
//! let file = media.upload("photo.jpg", image_bytes).await?;
//!
//! // Generate thumbnail
//! let thumb = media.thumbnail(&file.id, 150, 150).await?;
//!
//! // Get URL
//! let url = media.url(&file.id).await;
//! # Ok(())
//! # }
//! ```

use thiserror::Error;

pub mod editor;
pub mod media;
pub mod revisions;

pub use editor::{ContentSanitizer, EditorConfig};
pub use media::{MediaFile, MediaLibrary, StorageBackend};
pub use revisions::{Revision, RevisionManager};

/// CMS errors
#[derive(Error, Debug)]
pub enum CmsError {
    #[error("Media error: {0}")]
    MediaError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Revision error: {0}")]
    RevisionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image error: {0}")]
    ImageLibError(#[from] image::ImageError),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type CmsResult<T> = Result<T, CmsError>;
