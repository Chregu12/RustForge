//! File streaming support for efficient large file handling

use crate::{Storage, StorageResult};
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream of file chunks from storage
pub struct FileStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>,
    content_type: Option<String>,
    file_name: Option<String>,
}

impl FileStream {
    /// Create a new file stream
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(stream),
            content_type: None,
            file_name: None,
        }
    }

    /// Set the content type
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set the file name
    pub fn with_file_name(mut self, file_name: impl Into<String>) -> Self {
        self.file_name = Some(file_name.into());
        self
    }

    /// Create a file stream from storage
    pub async fn from_storage<S: Storage>(storage: &S, path: &str) -> StorageResult<Self> {
        let contents = storage.get(path).await?;
        let stream = futures::stream::once(async move { Ok(Bytes::from(contents)) });

        Ok(Self::new(stream))
    }

    /// Convert to axum Response with appropriate headers
    pub fn into_response(self) -> Response {
        let mut response = Response::builder();

        if let Some(ref content_type) = self.content_type {
            response = response.header("Content-Type", content_type.clone());
        } else {
            response = response.header("Content-Type", "application/octet-stream");
        }

        if let Some(ref file_name) = self.file_name {
            response = response.header(
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", file_name),
            );
        }

        response
            .body(Body::from_stream(self))
            .unwrap()
            .into_response()
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Helper to detect content type from file extension
pub fn detect_content_type(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or("");

    match extension.to_lowercase().as_str() {
        // Images
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",

        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Text
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",

        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",

        // Video
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",

        // Audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",

        // Default
        _ => "application/octet-stream",
    }
}

/// Extract file name from path
pub fn extract_file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_content_type() {
        assert_eq!(detect_content_type("test.jpg"), "image/jpeg");
        assert_eq!(detect_content_type("test.png"), "image/png");
        assert_eq!(detect_content_type("test.pdf"), "application/pdf");
        assert_eq!(detect_content_type("test.txt"), "text/plain");
        assert_eq!(detect_content_type("test.json"), "application/json");
        assert_eq!(detect_content_type("test.mp4"), "video/mp4");
        assert_eq!(
            detect_content_type("test.unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_extract_file_name() {
        assert_eq!(extract_file_name("test.txt"), "test.txt");
        assert_eq!(extract_file_name("path/to/test.txt"), "test.txt");
        assert_eq!(extract_file_name("path/to/deep/test.txt"), "test.txt");
    }

    #[tokio::test]
    async fn test_file_stream() {
        let data = Bytes::from("Hello, World!");
        let stream = futures::stream::once(async move { Ok::<_, std::io::Error>(data) });

        let file_stream = FileStream::new(stream)
            .with_content_type("text/plain")
            .with_file_name("test.txt");

        assert_eq!(file_stream.content_type, Some("text/plain".to_string()));
        assert_eq!(file_stream.file_name, Some("test.txt".to_string()));
    }
}
