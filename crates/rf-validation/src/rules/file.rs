//! File & image validation rules for uploaded files.
//!
//! Unlike the other rules in this crate, these do **not** operate over the JSON
//! `Value` field map: an uploaded file lives outside that map (in the request's
//! multipart body). They instead inspect a file's *metadata* — its declared MIME
//! type and byte size — and are wired by the `validate!` macro against the
//! current request's `rf_request::file(name)`.
//!
//! They are deliberately framework-agnostic (they take `Option<&str>` /`usize`
//! rather than a concrete `UploadedFile`) so that `rf-validation` need not depend
//! on `rf-request` — `rf-request` already depends on `rf-validation`, and the
//! reverse would be a cycle.

/// Byte-size helper: `kb(1)` is `1024` bytes.
///
/// Exists because `1.kb` / `5.mb` do not lex as a single token, so the size DSL
/// spells limits as calls: `max(mb(5))`, `min(kb(1))`.
pub const fn kb(n: u64) -> u64 {
    n * 1024
}

/// Byte-size helper: `mb(5)` is `5 * 1024 * 1024` bytes.
pub const fn mb(n: u64) -> u64 {
    n * 1024 * 1024
}

/// Requires the value to be an image upload (a `image/*` MIME type).
pub struct ImageRule;

impl Default for ImageRule {
    fn default() -> Self {
        Self
    }
}

impl ImageRule {
    /// Construct the rule.
    pub fn new() -> Self {
        Self
    }

    /// Check a file's declared content type. `Ok(())` when it is an image.
    pub fn check(&self, content_type: Option<&str>) -> Result<(), String> {
        match content_type {
            Some(ct) if ct.trim().to_ascii_lowercase().starts_with("image/") => Ok(()),
            Some(ct) => Err(format!(
                "This field must be an image (got content type `{}`)",
                ct.trim()
            )),
            None => Err("This field must be an image (no content type provided)".to_string()),
        }
    }
}

/// Restricts an upload to an explicit allow-list of MIME types.
///
/// Matching is case-insensitive. Each allowed entry matches either the full MIME
/// type (`image/png`) or, when written as a bare group (`image`), any subtype of
/// that group (`image/*`).
pub struct MimeRule {
    allowed: Vec<String>,
}

impl MimeRule {
    /// Build the rule from an allow-list, e.g. `MimeRule::new(["image/png", "image/jpeg"])`.
    pub fn new<I, S>(allowed: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            allowed: allowed
                .into_iter()
                .map(|s| s.into().trim().to_ascii_lowercase())
                .collect(),
        }
    }

    /// Check a file's declared content type against the allow-list.
    pub fn check(&self, content_type: Option<&str>) -> Result<(), String> {
        let ct = match content_type {
            Some(c) => c.trim().to_ascii_lowercase(),
            None => {
                return Err(format!(
                    "This field must be one of the allowed types: {}",
                    self.allowed.join(", ")
                ))
            }
        };
        let group = ct.split('/').next().unwrap_or("");
        let ok = self
            .allowed
            .iter()
            .any(|a| a == &ct || (!a.contains('/') && a == group));
        if ok {
            Ok(())
        } else {
            Err(format!(
                "This field must be one of the allowed types: {} (got `{}`)",
                self.allowed.join(", "),
                ct
            ))
        }
    }
}

/// Enforces minimum and/or maximum byte size on an upload.
///
/// Bounds are inclusive and expressed in bytes; use [`kb`]/[`mb`] for readable
/// limits (`FileSizeRule::new().max(mb(5))`).
#[derive(Default)]
pub struct FileSizeRule {
    min: Option<u64>,
    max: Option<u64>,
}

impl FileSizeRule {
    /// A size rule with no bounds yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum size in bytes.
    pub fn min(mut self, bytes: u64) -> Self {
        self.min = Some(bytes);
        self
    }

    /// Set the maximum size in bytes.
    pub fn max(mut self, bytes: u64) -> Self {
        self.max = Some(bytes);
        self
    }

    /// Check a file's byte size against the configured bounds.
    pub fn check(&self, size: usize) -> Result<(), String> {
        let size = size as u64;
        if let Some(min) = self.min {
            if size < min {
                return Err(format!(
                    "This file is too small: {} (minimum {} bytes)",
                    human_size(size),
                    min
                ));
            }
        }
        if let Some(max) = self.max {
            if size > max {
                return Err(format!(
                    "This file is too large: {} (maximum {} bytes)",
                    human_size(size),
                    max
                ));
            }
        }
        Ok(())
    }
}

/// Format a byte count into a short human-readable string for error messages.
fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kb_mb_helpers() {
        assert_eq!(kb(1), 1024);
        assert_eq!(kb(2), 2048);
        assert_eq!(mb(1), 1024 * 1024);
        assert_eq!(mb(5), 5 * 1024 * 1024);
    }

    #[test]
    fn image_rule_accepts_image_mimes() {
        let rule = ImageRule::new();
        assert!(rule.check(Some("image/png")).is_ok());
        assert!(rule.check(Some("IMAGE/JPEG")).is_ok());
        assert!(rule.check(Some(" image/gif ")).is_ok());
    }

    #[test]
    fn image_rule_rejects_non_images() {
        let rule = ImageRule::new();
        assert!(rule.check(Some("application/pdf")).is_err());
        assert!(rule.check(Some("text/plain")).is_err());
        assert!(rule.check(None).is_err());
    }

    #[test]
    fn mime_rule_matches_exact_and_group() {
        let exact = MimeRule::new(["image/png", "image/jpeg"]);
        assert!(exact.check(Some("image/png")).is_ok());
        assert!(exact.check(Some("image/jpeg")).is_ok());
        assert!(exact.check(Some("image/gif")).is_err());

        let group = MimeRule::new(["application"]);
        assert!(group.check(Some("application/pdf")).is_ok());
        assert!(group.check(Some("application/zip")).is_ok());
        assert!(group.check(Some("image/png")).is_err());
        assert!(group.check(None).is_err());
    }

    #[test]
    fn file_size_rule_enforces_bounds() {
        let rule = FileSizeRule::new().min(kb(1)).max(mb(5));
        assert!(rule.check(kb(1) as usize).is_ok());
        assert!(rule.check(mb(5) as usize).is_ok());
        assert!(rule.check(mb(1) as usize).is_ok());
        // Too small
        assert!(rule.check(512).is_err());
        // Too large
        assert!(rule.check((mb(5) + 1) as usize).is_err());
    }

    #[test]
    fn file_size_rule_open_bounds() {
        // Only a max.
        let max_only = FileSizeRule::new().max(mb(1));
        assert!(max_only.check(0).is_ok());
        assert!(max_only.check((mb(1) + 1) as usize).is_err());
        // Only a min.
        let min_only = FileSizeRule::new().min(kb(1));
        assert!(min_only.check(kb(1) as usize).is_ok());
        assert!(min_only.check(0).is_err());
        // No bounds: anything passes.
        assert!(FileSizeRule::new().check(999_999).is_ok());
    }
}
