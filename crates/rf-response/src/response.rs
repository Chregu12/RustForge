//! Response builder implementation

use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
};
use bytes::Bytes;
use futures::Stream;
use serde::Serialize;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Response builder
pub struct Response;

impl Response {
    /// Create a JSON response
    pub fn json<T: Serialize>(data: &T) -> ResponseBuilder {
        ResponseBuilder::new().json(data).status(StatusCode::OK)
    }

    /// Create a redirect response
    pub fn redirect(url: impl Into<String>) -> ResponseBuilder {
        ResponseBuilder::new().redirect(url)
    }

    /// Redirect "back" to the previous page.
    ///
    /// Without access to the incoming request's `Referer` header this falls back
    /// to the site root (`/`). Use [`Response::back_or`] to choose a different
    /// fallback destination.
    pub fn back() -> ResponseBuilder {
        ResponseBuilder::new().redirect("/")
    }

    /// Redirect "back", using `fallback` as the destination when no previous
    /// page is known.
    pub fn back_or(fallback: impl Into<String>) -> ResponseBuilder {
        ResponseBuilder::new().redirect(fallback)
    }

    /// Create a file download response
    pub fn download(path: impl Into<String>, filename: impl Into<String>) -> ResponseBuilder {
        ResponseBuilder::new().download(path, filename)
    }

    /// Create a streaming response
    pub fn stream<S>(stream: S) -> ResponseBuilder
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        ResponseBuilder::new().stream(stream)
    }

    /// Create a no-content response
    pub fn no_content() -> ResponseBuilder {
        ResponseBuilder::new().status(StatusCode::NO_CONTENT)
    }

    /// Create a plain text response
    pub fn text(text: impl Into<String>) -> ResponseBuilder {
        ResponseBuilder::new().text(text).status(StatusCode::OK)
    }

    /// Render an HTML view template and return it as a `text/html` response.
    ///
    /// Loads `resources/views/<name>.blade.html` (dots in `name` become path
    /// separators, e.g. `"users.index"`), interpolates `{{ var }}` placeholders
    /// from `data`, and returns the rendered HTML. See [`crate::view`] for the
    /// free-function form and rendering details.
    ///
    /// ```no_run
    /// use rf_response::Response;
    /// let resp = Response::view("home", serde_json::json!({ "title": "Welcome" }));
    /// ```
    pub fn view<T: Serialize>(name: impl Into<String>, data: T) -> crate::view::ViewResponse {
        crate::view::ViewResponse::new(name, data)
    }
}

/// Response builder for constructing responses
pub struct ResponseBuilder {
    status: StatusCode,
    headers: HeaderMap,
    body: Option<Body>,
    flash_data: Vec<(String, String)>,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: None,
            flash_data: Vec::new(),
        }
    }

    /// Set the status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add a header
    pub fn header(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        if let (Ok(name), Ok(val)) = (
            key.as_ref().parse::<header::HeaderName>(),
            HeaderValue::from_str(value.as_ref()),
        ) {
            self.headers.insert(name, val);
        }
        self
    }

    /// Set JSON body
    pub fn json<T: Serialize>(mut self, data: &T) -> Self {
        if let Ok(json) = serde_json::to_vec(data) {
            self.body = Some(Body::from(json));
            self = self.header("content-type", "application/json");
        }
        self
    }

    /// Set plain text body
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body = Some(Body::from(text.into()));
        self = self.header("content-type", "text/plain");
        self
    }

    /// Set redirect
    pub fn redirect(mut self, url: impl Into<String>) -> Self {
        self.status = StatusCode::FOUND;
        self = self.header("location", url.into());
        self
    }

    /// Set file download
    pub fn download(mut self, path: impl Into<String>, filename: impl Into<String>) -> Self {
        let path = path.into();
        let filename = filename.into();

        // Set the download headers regardless of whether the file exists, so a
        // caller can always see what was requested.
        self = self.header(
            "content-disposition",
            format!("attachment; filename=\"{}\"", filename),
        );
        self = self.header("content-type", "application/octet-stream");

        // Serve the real file bytes off disk. A missing/unreadable file becomes a
        // 404 with an empty body rather than fabricated content.
        match std::fs::read(&path) {
            Ok(bytes) => {
                self.body = Some(Body::from(bytes));
            }
            Err(_) => {
                self.status = StatusCode::NOT_FOUND;
                self.body = Some(Body::empty());
            }
        }
        self
    }

    /// Set streaming body
    pub fn stream<S>(mut self, stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
    {
        self.body = Some(Body::from_stream(stream));
        self
    }

    /// Add flash data (for redirects)
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.flash_data.push((key.into(), value.into()));
        self
    }

    /// Build the response
    pub fn build(self) -> AxumResponse {
        let mut response = AxumResponse::new(self.body.unwrap_or_else(Body::empty));
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;

        // In a real implementation, flash data would be stored in session
        // For now, we'll add it as a custom header for demonstration
        if !self.flash_data.is_empty() {
            for (key, value) in self.flash_data {
                if let Ok(header_value) = HeaderValue::from_str(&value) {
                    response.headers_mut().insert(
                        header::HeaderName::from_bytes(format!("x-flash-{}", key).as_bytes())
                            .unwrap_or(header::HeaderName::from_static("x-flash-data")),
                        header_value,
                    );
                }
            }
        }

        response
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> AxumResponse {
        self.build()
    }
}

// ============================================================================
// Global helper functions (Laravel-style free functions)
//
// These mirror the `Response` constructors as bare functions so handlers can
// write `json(data)` / `download(path)` directly. Each returns a
// [`ResponseBuilder`], which implements [`IntoResponse`].
// ============================================================================

/// Build a JSON response from any [`Serialize`] value.
///
/// ```
/// use rf_response::json;
/// let resp = json(serde_json::json!({"ok": true}));
/// ```
pub fn json<T: Serialize>(data: T) -> ResponseBuilder {
    Response::json(&data)
}

/// Redirect to `url` (302 Found).
///
/// ```
/// use rf_response::redirect;
/// let resp = redirect("/dashboard");
/// ```
pub fn redirect(url: impl Into<String>) -> ResponseBuilder {
    Response::redirect(url)
}

/// Redirect "back" to the previous page, falling back to the site root (`/`).
///
/// ```
/// use rf_response::back;
/// let resp = back();
/// ```
pub fn back() -> ResponseBuilder {
    Response::back()
}

/// Serve a file download. The `Content-Disposition` filename is derived from the
/// path's final component (falling back to `"download"`).
///
/// ```no_run
/// use rf_response::download;
/// let resp = download("/var/www/report.pdf");
/// ```
pub fn download(path: impl Into<String>) -> ResponseBuilder {
    let path = path.into();
    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("download")
        .to_string();
    Response::download(path, filename)
}

/// Stream body wrapper
pub struct StreamBody<S> {
    stream: S,
}

impl<S> StreamBody<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S> Stream for StreamBody<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_response() {
        let response = Response::json(&serde_json::json!({"status": "ok"})).build();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("content-type").is_some());
    }

    #[test]
    fn test_redirect_response() {
        let response = Response::redirect("/dashboard").build();

        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(response.headers().get("location").unwrap(), "/dashboard");
    }

    #[test]
    fn test_download_response() {
        let response = Response::download("/path/to/file.pdf", "invoice.pdf").build();

        let content_disposition = response
            .headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap();

        assert!(content_disposition.contains("invoice.pdf"));
        assert!(content_disposition.contains("attachment"));
    }

    #[test]
    fn test_no_content_response() {
        let response = Response::no_content().build();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[test]
    fn test_flash_data() {
        let response = Response::redirect("/dashboard")
            .with("success", "Profile updated!")
            .build();

        assert_eq!(response.status(), StatusCode::FOUND);
        // Flash data would be in headers (in real impl, it would be in session)
    }
}
