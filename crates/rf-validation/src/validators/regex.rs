//! Regex validation

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;

static REGEX_CACHE: Lazy<Mutex<HashMap<String, Regex>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Validate string against regex pattern
pub fn validate_regex(value: &str, pattern: &str) -> bool {
    let mut cache = REGEX_CACHE.lock().unwrap();

    let regex = cache
        .entry(pattern.to_string())
        .or_insert_with(|| Regex::new(pattern).unwrap_or_else(|_| Regex::new("^$").unwrap()));

    regex.is_match(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_validation() {
        assert!(validate_regex("abc123", r"^[a-z]+\d+$"));
        assert!(!validate_regex("123abc", r"^[a-z]+\d+$"));
        assert!(validate_regex("hello", r"^h.*o$"));
    }
}
