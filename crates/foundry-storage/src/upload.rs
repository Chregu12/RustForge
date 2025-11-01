//! Upload manager

use bytes::Bytes;
use anyhow::Result;
use std::path::PathBuf;

pub struct UploadManager {
    max_file_size: u64,
    allowed_extensions: Vec<String>,
}

impl UploadManager {
    pub fn new() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_extensions: vec![],
        }
    }

    pub fn max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = size;
        self
    }

    pub fn allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = extensions;
        self
    }

    pub fn validate(&self, filename: &str, size: u64) -> Result<()> {
        if size > self.max_file_size {
            anyhow::bail!("File too large");
        }

        if !self.allowed_extensions.is_empty() {
            let ext = PathBuf::from(filename)
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());

            if let Some(ext) = ext {
                if !self.allowed_extensions.contains(&ext) {
                    anyhow::bail!("File extension not allowed");
                }
            } else {
                anyhow::bail!("No file extension");
            }
        }

        Ok(())
    }

    pub async fn store(&self, filename: &str, content: Bytes) -> Result<String> {
        self.validate(filename, content.len() as u64)?;
        // Store file
        Ok(filename.to_string())
    }
}

impl Default for UploadManager {
    fn default() -> Self {
        Self::new()
    }
}
