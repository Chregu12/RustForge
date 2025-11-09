//! Email attachment types

use serde::{Deserialize, Serialize};

/// Email attachment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attachment {
    /// Filename
    pub filename: String,

    /// Content type (MIME type)
    pub content_type: String,

    /// Attachment data
    pub data: Vec<u8>,
}

impl Attachment {
    /// Create new attachment
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::Attachment;
    ///
    /// let data = b"file contents".to_vec();
    /// let attachment = Attachment::new("document.txt", "text/plain", data);
    /// assert_eq!(attachment.filename, "document.txt");
    /// ```
    pub fn new(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            data,
        }
    }

    /// Create attachment from file path
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_mail::Attachment;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let attachment = Attachment::from_file("report.pdf", "application/pdf").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_file(
        path: impl AsRef<std::path::Path>,
        content_type: impl Into<String>,
    ) -> Result<Self, std::io::Error> {
        let data = tokio::fs::read(&path).await?;
        let filename = path
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();

        Ok(Self::new(filename, content_type, data))
    }

    /// Size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_new() {
        let data = b"Hello, World!".to_vec();
        let attachment = Attachment::new("hello.txt", "text/plain", data.clone());

        assert_eq!(attachment.filename, "hello.txt");
        assert_eq!(attachment.content_type, "text/plain");
        assert_eq!(attachment.data, data);
    }

    #[test]
    fn test_attachment_size() {
        let data = b"Hello, World!".to_vec();
        let attachment = Attachment::new("hello.txt", "text/plain", data);

        assert_eq!(attachment.size(), 13);
    }
}
