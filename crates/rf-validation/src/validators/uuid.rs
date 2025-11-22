//! UUID validation

use once_cell::sync::Lazy;
use regex::Regex;

static UUID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .unwrap()
});

/// Validate UUID
pub fn validate_uuid(uuid: &str) -> bool {
    UUID_REGEX.is_match(uuid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_uuids() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(validate_uuid("6ba7b810-9dad-11d1-80b4-00c04fd430c8"));
    }

    #[test]
    fn test_invalid_uuids() {
        assert!(!validate_uuid(""));
        assert!(!validate_uuid("invalid"));
        assert!(!validate_uuid("550e8400-e29b-41d4-a716"));
        assert!(!validate_uuid("550e8400e29b41d4a716446655440000"));
    }
}
