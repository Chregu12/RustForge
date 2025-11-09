//! Email address types

use serde::{Deserialize, Serialize};

/// Email address with optional name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Address {
    /// Email address
    pub email: String,

    /// Display name
    pub name: Option<String>,
}

impl Address {
    /// Create new address with email only
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::Address;
    ///
    /// let addr = Address::new("user@example.com");
    /// assert_eq!(addr.email, "user@example.com");
    /// assert_eq!(addr.name, None);
    /// ```
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    /// Create new address with email and name
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::Address;
    ///
    /// let addr = Address::with_name("user@example.com", "John Doe");
    /// assert_eq!(addr.email, "user@example.com");
    /// assert_eq!(addr.name, Some("John Doe".to_string()));
    /// ```
    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }

    /// Format address for display
    pub fn format(&self) -> String {
        match &self.name {
            Some(name) => format!("{} <{}>", name, self.email),
            None => self.email.clone(),
        }
    }
}

impl From<&str> for Address {
    fn from(email: &str) -> Self {
        Self::new(email)
    }
}

impl From<String> for Address {
    fn from(email: String) -> Self {
        Self::new(email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_new() {
        let addr = Address::new("test@example.com");
        assert_eq!(addr.email, "test@example.com");
        assert_eq!(addr.name, None);
    }

    #[test]
    fn test_address_with_name() {
        let addr = Address::with_name("test@example.com", "Test User");
        assert_eq!(addr.email, "test@example.com");
        assert_eq!(addr.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_address_format() {
        let addr1 = Address::new("test@example.com");
        assert_eq!(addr1.format(), "test@example.com");

        let addr2 = Address::with_name("test@example.com", "Test User");
        assert_eq!(addr2.format(), "Test User <test@example.com>");
    }

    #[test]
    fn test_address_from_str() {
        let addr: Address = "test@example.com".into();
        assert_eq!(addr.email, "test@example.com");
    }
}
