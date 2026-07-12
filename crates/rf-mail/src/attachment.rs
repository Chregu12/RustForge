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
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let attachment = Attachment::from_file("report.pdf")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, std::io::Error> {
        let data = std::fs::read(&path)?;
        let filename = path
            .as_ref()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("attachment")
            .to_string();

        // Guess content type from extension
        let content_type = guess_content_type(&filename);

        Ok(Self::new(filename, content_type, data))
    }

    /// Create attachment from data
    pub fn from_data(data: Vec<u8>, filename: String, content_type: String) -> Self {
        Self::new(filename, content_type, data)
    }

    /// Size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Guess content type from filename extension
fn guess_content_type(filename: &str) -> String {
    let extension = filename.split('.').next_back().unwrap_or("").to_lowercase();

    match extension.as_str() {
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "zip" => "application/zip",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "csv" => "text/csv",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
    .to_string()
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
