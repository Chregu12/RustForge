//! WYSIWYG editor integration and content sanitization

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::CmsResult;

/// Editor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Editor type (tinymce, ckeditor, etc.)
    pub editor_type: EditorType,

    /// Toolbar configuration
    pub toolbar: Vec<String>,

    /// Plugins
    pub plugins: Vec<String>,

    /// Height in pixels
    pub height: Option<u32>,

    /// Custom options
    pub options: serde_json::Value,
}

/// Editor type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EditorType {
    TinyMCE,
    CKEditor,
    Quill,
}

impl EditorConfig {
    /// Create TinyMCE configuration
    pub fn tinymce() -> Self {
        Self {
            editor_type: EditorType::TinyMCE,
            toolbar: vec![
                "undo redo".to_string(),
                "bold italic underline".to_string(),
                "alignleft aligncenter alignright".to_string(),
                "bullist numlist".to_string(),
                "link image".to_string(),
            ],
            plugins: vec![
                "advlist".to_string(),
                "autolink".to_string(),
                "lists".to_string(),
                "link".to_string(),
                "image".to_string(),
                "charmap".to_string(),
                "preview".to_string(),
                "searchreplace".to_string(),
                "code".to_string(),
            ],
            height: Some(400),
            options: serde_json::json!({}),
        }
    }

    /// Create CKEditor configuration
    pub fn ckeditor() -> Self {
        Self {
            editor_type: EditorType::CKEditor,
            toolbar: vec![
                "heading".to_string(),
                "bold italic link".to_string(),
                "bulletedList numberedList".to_string(),
                "imageUpload blockQuote".to_string(),
                "undo redo".to_string(),
            ],
            plugins: vec![
                "Essentials".to_string(),
                "Bold".to_string(),
                "Italic".to_string(),
                "Link".to_string(),
                "List".to_string(),
                "Paragraph".to_string(),
            ],
            height: Some(400),
            options: serde_json::json!({}),
        }
    }

    /// Generate initialization JavaScript
    pub fn init_script(&self, selector: &str) -> String {
        // Escape selector for safe embedding in JavaScript string literals
        let escaped_selector = selector
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r");

        match self.editor_type {
            EditorType::TinyMCE => {
                format!(
                    r#"tinymce.init({{
    selector: '{}',
    toolbar: '{}',
    plugins: '{}',
    height: {}
}});"#,
                    escaped_selector,
                    self.toolbar.join(" | "),
                    self.plugins.join(" "),
                    self.height.unwrap_or(400)
                )
            }
            EditorType::CKEditor => {
                format!(
                    r#"ClassicEditor.create(document.querySelector('{}'), {{
    toolbar: [{}],
    height: '{}'
}});"#,
                    escaped_selector,
                    self.toolbar
                        .iter()
                        .map(|t| format!("'{}'", t))
                        .collect::<Vec<_>>()
                        .join(", "),
                    self.height.unwrap_or(400)
                )
            }
            EditorType::Quill => {
                format!(
                    r#"new Quill('{}', {{
    theme: 'snow',
    modules: {{
        toolbar: [{}]
    }}
}});"#,
                    escaped_selector,
                    self.toolbar
                        .iter()
                        .map(|t| format!("'{}'", t))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

/// Content sanitizer for HTML content
pub struct ContentSanitizer {
    allowed_tags: Vec<String>,
    allowed_attributes: Vec<String>,
}

impl ContentSanitizer {
    /// Create a new sanitizer with default safe tags
    pub fn new() -> Self {
        Self {
            allowed_tags: vec![
                "p".to_string(),
                "br".to_string(),
                "strong".to_string(),
                "b".to_string(),
                "em".to_string(),
                "i".to_string(),
                "u".to_string(),
                "a".to_string(),
                "ul".to_string(),
                "ol".to_string(),
                "li".to_string(),
                "blockquote".to_string(),
                "h1".to_string(),
                "h2".to_string(),
                "h3".to_string(),
                "h4".to_string(),
                "h5".to_string(),
                "h6".to_string(),
                "img".to_string(),
                "span".to_string(),
                "div".to_string(),
            ],
            allowed_attributes: vec![
                "href".to_string(),
                "src".to_string(),
                "alt".to_string(),
                "title".to_string(),
                "class".to_string(),
                "id".to_string(),
            ],
        }
    }

    /// Allow additional tag
    pub fn allow_tag(mut self, tag: impl Into<String>) -> Self {
        self.allowed_tags.push(tag.into());
        self
    }

    /// Allow additional attribute
    pub fn allow_attribute(mut self, attr: impl Into<String>) -> Self {
        self.allowed_attributes.push(attr.into());
        self
    }

    /// Sanitize HTML content
    pub fn sanitize(&self, html: &str) -> CmsResult<String> {
        let mut result = html.to_string();

        // Remove script tags
        let script_re = Regex::new(r"(?i)<script[^>]*>.*?</script>").unwrap();
        result = script_re.replace_all(&result, "").to_string();

        // Remove onclick/onerror/etc handlers
        let event_re = Regex::new(r#"(?i)\s+on\w+\s*=\s*["'][^"']*["']"#).unwrap();
        result = event_re.replace_all(&result, "").to_string();

        // Remove javascript: protocols
        let js_protocol_re = Regex::new(r"(?i)javascript:").unwrap();
        result = js_protocol_re.replace_all(&result, "").to_string();

        // Remove data: URLs (potential XSS vector)
        let data_url_re = Regex::new(r#"(?i)src\s*=\s*["']data:"#).unwrap();
        result = data_url_re.replace_all(&result, "").to_string();

        // Remove style tags
        let style_re = Regex::new(r"(?i)<style[^>]*>.*?</style>").unwrap();
        result = style_re.replace_all(&result, "").to_string();

        Ok(result)
    }

    /// Strip all HTML tags
    pub fn strip_tags(&self, html: &str) -> String {
        let tag_re = Regex::new(r"<[^>]*>").unwrap();
        tag_re.replace_all(html, "").to_string()
    }

    /// Extract plain text from HTML
    pub fn to_plain_text(&self, html: &str) -> String {
        let mut text = self.strip_tags(html);

        // Decode HTML entities
        text = text.replace("&nbsp;", " ");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&amp;", "&");
        text = text.replace("&quot;", "\"");
        text = text.replace("&#39;", "'");

        // Normalize whitespace
        let ws_re = Regex::new(r"\s+").unwrap();
        text = ws_re.replace_all(&text, " ").to_string();

        text.trim().to_string()
    }
}

impl Default for ContentSanitizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tinymce_config() {
        let config = EditorConfig::tinymce();
        assert!(matches!(config.editor_type, EditorType::TinyMCE));
        assert!(!config.toolbar.is_empty());
        assert!(!config.plugins.is_empty());
    }

    #[test]
    fn test_ckeditor_config() {
        let config = EditorConfig::ckeditor();
        assert!(matches!(config.editor_type, EditorType::CKEditor));
        assert!(!config.toolbar.is_empty());
    }

    #[test]
    fn test_tinymce_init_script() {
        let config = EditorConfig::tinymce();
        let script = config.init_script("#editor");

        assert!(script.contains("tinymce.init"));
        assert!(script.contains("#editor"));
    }

    #[test]
    fn test_sanitizer_removes_scripts() {
        let sanitizer = ContentSanitizer::new();
        let html = r#"<p>Safe content</p><script>alert('xss')</script>"#;
        let clean = sanitizer.sanitize(html).unwrap();

        assert!(clean.contains("Safe content"));
        assert!(!clean.contains("script"));
        assert!(!clean.contains("alert"));
    }

    #[test]
    fn test_sanitizer_removes_event_handlers() {
        let sanitizer = ContentSanitizer::new();
        let html = r#"<div onclick="alert('xss')">Click me</div>"#;
        let clean = sanitizer.sanitize(html).unwrap();

        assert!(clean.contains("Click me"));
        assert!(!clean.contains("onclick"));
    }

    #[test]
    fn test_sanitizer_removes_javascript_protocol() {
        let sanitizer = ContentSanitizer::new();
        let html = r#"<a href="javascript:alert('xss')">Link</a>"#;
        let clean = sanitizer.sanitize(html).unwrap();

        assert!(!clean.contains("javascript:"));
    }

    #[test]
    fn test_strip_tags() {
        let sanitizer = ContentSanitizer::new();
        let html = "<p>Hello <strong>World</strong>!</p>";
        let text = sanitizer.strip_tags(html);

        assert_eq!(text, "Hello World!");
    }

    #[test]
    fn test_to_plain_text() {
        let sanitizer = ContentSanitizer::new();
        let html = "<p>Hello&nbsp;<strong>World</strong>!</p>";
        let text = sanitizer.to_plain_text(html);

        assert_eq!(text, "Hello World!");
    }

    #[test]
    fn test_html_entity_decoding() {
        let sanitizer = ContentSanitizer::new();
        let html = "&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;";
        let text = sanitizer.to_plain_text(html);

        assert_eq!(text, "<script>alert(\"xss\")</script>");
    }
}
