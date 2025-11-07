use serde::{Deserialize, Serialize};
use std::path::Path;

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub inline: bool,
    pub content_id: Option<String>,
}

impl Attachment {
    pub fn builder() -> AttachmentBuilder {
        AttachmentBuilder::default()
    }

    pub fn from_bytes(filename: impl Into<String>, content_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            data,
            inline: false,
            content_id: None,
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, AttachmentError> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(AttachmentError::InvalidFilename)?
            .to_string();

        let content_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(Self::from_bytes(filename, content_type, data))
    }

    pub fn as_inline(mut self, content_id: impl Into<String>) -> Self {
        self.inline = true;
        self.content_id = Some(content_id.into());
        self
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Default)]
pub struct AttachmentBuilder {
    filename: Option<String>,
    content_type: Option<String>,
    data: Option<Vec<u8>>,
    inline: bool,
    content_id: Option<String>,
}

impl AttachmentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    pub fn inline(mut self, content_id: impl Into<String>) -> Self {
        self.inline = true;
        self.content_id = Some(content_id.into());
        self
    }

    pub fn from_path(mut self, path: impl AsRef<Path>) -> Result<Self, AttachmentError> {
        let path = path.as_ref();
        self.data = Some(std::fs::read(path)?);

        if self.filename.is_none() {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or(AttachmentError::InvalidFilename)?;
            self.filename = Some(filename.to_string());
        }

        if self.content_type.is_none() {
            let content_type = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            self.content_type = Some(content_type);
        }

        Ok(self)
    }

    pub fn build(self) -> Result<Attachment, AttachmentError> {
        Ok(Attachment {
            filename: self.filename.ok_or(AttachmentError::MissingFilename)?,
            content_type: self.content_type.ok_or(AttachmentError::MissingContentType)?,
            data: self.data.ok_or(AttachmentError::MissingData)?,
            inline: self.inline,
            content_id: self.content_id,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AttachmentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid filename")]
    InvalidFilename,

    #[error("Missing filename")]
    MissingFilename,

    #[error("Missing content type")]
    MissingContentType,

    #[error("Missing data")]
    MissingData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_from_bytes() {
        let data = b"test content".to_vec();
        let attachment = Attachment::from_bytes("test.txt", "text/plain", data.clone());

        assert_eq!(attachment.filename, "test.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(attachment.data, data);
        assert!(!attachment.inline);
        assert_eq!(attachment.size(), 12);
    }

    #[test]
    fn test_attachment_inline() {
        let data = b"image data".to_vec();
        let attachment = Attachment::from_bytes("image.png", "image/png", data)
            .as_inline("img-1");

        assert!(attachment.inline);
        assert_eq!(attachment.content_id, Some("img-1".to_string()));
    }

    #[test]
    fn test_attachment_builder() {
        let data = b"test".to_vec();
        let attachment = AttachmentBuilder::new()
            .filename("test.txt")
            .content_type("text/plain")
            .data(data.clone())
            .build()
            .unwrap();

        assert_eq!(attachment.filename, "test.txt");
        assert_eq!(attachment.data, data);
    }
}
