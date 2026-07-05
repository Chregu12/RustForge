//! Real `view(name, data)` template response.
//!
//! Renders an HTML template from the conventional views directory
//! (`resources/views/<name>.blade.html`, or `.html`) by interpolating
//! `{{ var }}` placeholders from a [`serde::Serialize`] data value, and returns
//! it as a `text/html` HTTP response.
//!
//! This is a deliberately small but *real* renderer: it reads the actual
//! template file off disk and substitutes values from the supplied data. It is
//! NOT a full Blade engine — see [`crate::view::ViewResponse`] docs for what a
//! full engine (directives, layouts, components, loops) would add. For that,
//! the `rf-view` crate ships a Tera-backed engine (`rf_view::View`) that
//! requires a global glob-based init.
//!
//! # Example (handler)
//!
//! ```no_run
//! use rf_response::{view, Response};
//! use axum::response::IntoResponse;
//!
//! async fn home() -> impl IntoResponse {
//!     // renders resources/views/home.blade.html with {{ title }} filled in
//!     view("home", serde_json::json!({ "title": "Welcome" }))
//! }
//!
//! async fn home2() -> impl IntoResponse {
//!     Response::view("home", serde_json::json!({ "title": "Welcome" }))
//! }
//! ```

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

/// A rendered (or to-be-rendered) template response.
///
/// Rendering happens lazily in [`IntoResponse::into_response`] so the helper can
/// be returned directly from a handler. On success it yields a `200 OK` with a
/// `text/html; charset=utf-8` body; a missing template or unreadable file yields
/// a `500` carrying the real error (never fabricated HTML).
pub struct ViewResponse {
    name: String,
    data: Value,
}

impl ViewResponse {
    /// Build a view response for `name`, carrying `data` used to interpolate
    /// `{{ var }}` placeholders.
    pub fn new<T: Serialize>(name: impl Into<String>, data: T) -> Self {
        let data = serde_json::to_value(data).unwrap_or(Value::Null);
        Self {
            name: name.into(),
            data,
        }
    }

    /// Root directory templates are resolved against.
    ///
    /// Defaults to `resources/views` (Laravel-style), overridable via the
    /// `RUSTFORGE_VIEWS_PATH` environment variable.
    fn views_root() -> PathBuf {
        std::env::var_os("RUSTFORGE_VIEWS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("resources/views"))
    }

    /// Resolve a dotted/plain view name to an existing template file.
    ///
    /// `"users.index"` -> `resources/views/users/index.blade.html`
    /// (also tries `.html`, and the name verbatim if it already has an extension).
    fn resolve_path(name: &str) -> Option<PathBuf> {
        let root = Self::views_root();
        let mut candidates: Vec<PathBuf> = Vec::new();

        // If the caller already passed a concrete file (has .html), honour it.
        if name.ends_with(".html") {
            candidates.push(root.join(name));
        } else {
            let slashed = name.replace('.', "/");
            candidates.push(root.join(format!("{slashed}.blade.html")));
            candidates.push(root.join(format!("{slashed}.html")));
        }

        candidates.into_iter().find(|p| p.is_file())
    }

    /// Render the template to an HTML string, or return a human-readable error.
    pub fn render(&self) -> Result<String, String> {
        let path = Self::resolve_path(&self.name).ok_or_else(|| {
            format!(
                "view '{}' not found under {}",
                self.name,
                Self::views_root().display()
            )
        })?;

        let template = std::fs::read_to_string(&path)
            .map_err(|e| format!("failed to read view '{}': {e}", path.display()))?;

        Ok(interpolate(&template, &self.data))
    }
}

impl IntoResponse for ViewResponse {
    fn into_response(self) -> AxumResponse {
        match self.render() {
            Ok(html) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                html,
            )
                .into_response(),
            Err(e) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    e,
                )
                    .into_response()
            }
        }
    }
}

/// Interpolate `{{ path }}` placeholders in `template` from `data`.
///
/// - `path` may be a dotted lookup into nested objects (`user.name`).
/// - String values are inserted verbatim (after HTML-escaping); other JSON
///   values are rendered via their compact JSON string (numbers/bools as-is,
///   `null` as empty). Unknown paths render empty.
/// - Values are HTML-escaped, matching Blade's escaping `{{ }}` semantics.
fn interpolate(template: &str, data: &Value) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(close) = template[i + 2..].find("}}") {
                let expr = template[i + 2..i + 2 + close].trim();
                out.push_str(&html_escape(&lookup(data, expr)));
                i = i + 2 + close + 2;
                continue;
            }
        }
        // Not a placeholder: copy the current UTF-8 char.
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&template[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Length in bytes of a UTF-8 sequence given its leading byte.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Resolve a dotted path against a JSON value, producing a display string.
fn lookup(data: &Value, path: &str) -> String {
    let mut cur = data;
    for seg in path.split('.') {
        cur = match cur {
            Value::Object(map) => match map.get(seg) {
                Some(v) => v,
                None => return String::new(),
            },
            _ => return String::new(),
        };
    }
    match cur {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Minimal HTML entity escaping (matches Blade's escaped `{{ }}` output).
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Global `view(name, data)` helper.
///
/// Mirrors [`Response::view`](crate::Response::view) as a bare function so
/// handlers can write `view("home", data)` directly. Returns a [`ViewResponse`]
/// which implements [`axum::response::IntoResponse`].
pub fn view<T: Serialize>(name: impl Into<String>, data: T) -> ViewResponse {
    ViewResponse::new(name, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn interpolates_simple_and_nested() {
        let tpl = "<h1>{{ title }}</h1><p>{{ user.name }}</p>";
        let out = interpolate(tpl, &json!({"title": "Hi", "user": {"name": "Ada"}}));
        assert_eq!(out, "<h1>Hi</h1><p>Ada</p>");
    }

    #[test]
    fn escapes_html_and_handles_missing() {
        let tpl = "{{ danger }}|{{ missing }}|{{ n }}";
        let out = interpolate(tpl, &json!({"danger": "<b>&\"", "n": 42}));
        assert_eq!(out, "&lt;b&gt;&amp;&quot;||42");
    }

    #[test]
    fn preserves_non_placeholder_braces() {
        let tpl = "a { b } {c} {{x}}";
        let out = interpolate(tpl, &json!({"x": "X"}));
        assert_eq!(out, "a { b } {c} X");
    }
}
