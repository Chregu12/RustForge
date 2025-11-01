use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents an email address with optional display name
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub email: String,
    pub name: Option<String>,
}

impl Address {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }

    pub fn parse(s: &str) -> Result<Self, AddressParseError> {
        // Simple parser for "Name <email@example.com>" or "email@example.com"
        let s = s.trim();

        if let Some(start) = s.find('<') {
            if let Some(end) = s.find('>') {
                let name = s[..start].trim();
                let email = s[start + 1..end].trim();

                if email.is_empty() || !email.contains('@') {
                    return Err(AddressParseError::InvalidFormat);
                }

                return Ok(Self {
                    email: email.to_string(),
                    name: if name.is_empty() { None } else { Some(name.to_string()) },
                });
            }
        }

        // Simple email format
        if s.contains('@') {
            Ok(Self::new(s))
        } else {
            Err(AddressParseError::InvalidFormat)
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{} <{}>", name, self.email),
            None => write!(f, "{}", self.email),
        }
    }
}

impl From<String> for Address {
    fn from(email: String) -> Self {
        Self::new(email)
    }
}

impl From<&str> for Address {
    fn from(email: &str) -> Self {
        Self::new(email)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AddressParseError {
    #[error("Invalid address format")]
    InvalidFormat,
}

/// A list of email addresses
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddressList(Vec<Address>);

impl AddressList {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, address: Address) {
        self.0.push(address);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Address> {
        self.0.iter()
    }
}

impl From<Vec<Address>> for AddressList {
    fn from(addresses: Vec<Address>) -> Self {
        Self(addresses)
    }
}

impl From<Address> for AddressList {
    fn from(address: Address) -> Self {
        Self(vec![address])
    }
}

impl IntoIterator for AddressList {
    type Item = Address;
    type IntoIter = std::vec::IntoIter<Address>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_creation() {
        let addr = Address::new("user@example.com");
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, None);
    }

    #[test]
    fn test_address_with_name() {
        let addr = Address::with_name("user@example.com", "John Doe");
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_address_parse_simple() {
        let addr = Address::parse("user@example.com").unwrap();
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, None);
    }

    #[test]
    fn test_address_parse_with_name() {
        let addr = Address::parse("John Doe <user@example.com>").unwrap();
        assert_eq!(addr.email, "user@example.com");
        assert_eq!(addr.name, Some("John Doe".to_string()));
    }

    #[test]
    fn test_address_display() {
        let addr1 = Address::new("user@example.com");
        assert_eq!(addr1.to_string(), "user@example.com");

        let addr2 = Address::with_name("user@example.com", "John Doe");
        assert_eq!(addr2.to_string(), "John Doe <user@example.com>");
    }
}
