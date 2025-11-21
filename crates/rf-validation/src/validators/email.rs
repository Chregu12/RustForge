//! Email validation

use regex::Regex;
use once_cell::sync::Lazy;

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    ).unwrap()
});

/// Validate email address
pub fn validate_email(email: &str) -> bool {
    if email.is_empty() || email.len() > 254 {
        return false;
    }

    EMAIL_REGEX.is_match(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        assert!(validate_email("test@example.com"));
        assert!(validate_email("user.name+tag@example.co.uk"));
        assert!(validate_email("x@example.com"));
    }

    #[test]
    fn test_invalid_emails() {
        assert!(!validate_email(""));
        assert!(!validate_email("invalid"));
        assert!(!validate_email("@example.com"));
        assert!(!validate_email("user@"));
        assert!(!validate_email("user @example.com"));
    }
}
