use serde::{Deserialize, Serialize};

/// Email content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Html,
}

impl ContentType {
    pub fn as_str(&self) -> &str {
        match self {
            ContentType::Text => "text/plain",
            ContentType::Html => "text/html",
        }
    }
}

/// Email content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub text: Option<String>,
    pub html: Option<String>,
}

impl Content {
    pub fn new() -> Self {
        Self {
            text: None,
            html: None,
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            html: None,
        }
    }

    pub fn html(html: impl Into<String>) -> Self {
        Self {
            text: None,
            html: Some(html.into()),
        }
    }

    pub fn both(text: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            html: Some(html.into()),
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.html.is_none()
    }

    pub fn has_text(&self) -> bool {
        self.text.is_some()
    }

    pub fn has_html(&self) -> bool {
        self.html.is_some()
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_text() {
        let content = Content::text("Hello");
        assert!(content.has_text());
        assert!(!content.has_html());
        assert_eq!(content.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_content_html() {
        let content = Content::html("<p>Hello</p>");
        assert!(!content.has_text());
        assert!(content.has_html());
        assert_eq!(content.html, Some("<p>Hello</p>".to_string()));
    }

    #[test]
    fn test_content_both() {
        let content = Content::both("Hello", "<p>Hello</p>");
        assert!(content.has_text());
        assert!(content.has_html());
    }

    #[test]
    fn test_content_is_empty() {
        let content = Content::new();
        assert!(content.is_empty());

        let content = Content::text("Hello");
        assert!(!content.is_empty());
    }
}
