//! URL validation

use once_cell::sync::Lazy;
use regex::Regex;

static URL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\+.~#?&/=]*)$"
    ).unwrap()
});

/// Validate URL
pub fn validate_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }

    URL_REGEX.is_match(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_urls() {
        assert!(validate_url("http://example.com"));
        assert!(validate_url("https://example.com"));
        assert!(validate_url("https://www.example.com/path?query=value"));
    }

    #[test]
    fn test_invalid_urls() {
        assert!(!validate_url(""));
        assert!(!validate_url("invalid"));
        assert!(!validate_url("ftp://example.com"));
        assert!(!validate_url("example.com"));
    }
}
