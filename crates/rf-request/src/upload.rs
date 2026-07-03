//! In-flight uploaded file, parsed from a `multipart/form-data` request body.
//!
//! Unlike a stored file, this holds the raw bytes still in memory so the handler
//! can inspect and then persist them with [`UploadedFile::store`].

use bytes::Bytes;
use std::path::{Path, PathBuf};

/// A file received on a `multipart/form-data` request, before it is stored.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// The form field name this file was uploaded under (e.g. `"image"`).
    field: String,
    /// The client-provided original filename (e.g. `"photo.png"`), if any.
    filename: Option<String>,
    /// The declared MIME type (e.g. `"image/png"`), if any.
    content_type: Option<String>,
    /// The raw file bytes.
    bytes: Bytes,
}

impl UploadedFile {
    /// Construct an uploaded file (used by the request extractor).
    pub fn new(
        field: impl Into<String>,
        filename: Option<String>,
        content_type: Option<String>,
        bytes: Bytes,
    ) -> Self {
        Self {
            field: field.into(),
            filename,
            content_type,
            bytes,
        }
    }

    /// The form field name.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The original client filename, if provided.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// The declared MIME type, if provided.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// The size of the file in bytes.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// The raw file bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The file extension derived from the original filename, if any.
    pub fn extension(&self) -> Option<&str> {
        self.filename
            .as_deref()
            .and_then(|f| Path::new(f).extension())
            .and_then(|e| e.to_str())
    }

    /// Persist the file into `dir`, creating the directory if needed, and return
    /// the written path. The filename is the client-provided name, falling back
    /// to the field name when none was sent.
    pub fn store(&self, dir: impl AsRef<Path>) -> std::io::Result<PathBuf> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let name = self
            .filename
            .clone()
            .unwrap_or_else(|| self.field.clone());
        // Guard against path traversal: keep only the final path component.
        let safe = Path::new(&name)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.field.clone());
        let path = dir.join(safe);
        std::fs::write(&path, &self.bytes)?;
        Ok(path)
    }
}
