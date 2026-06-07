//! # Accessors & Mutators
//!
//! Provides Laravel-style accessors and mutators for virtual attributes and data transformation.
//! Accessors transform data when retrieving from the model, while mutators transform data
//! when setting values.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//!
//! #[derive(Clone, Debug)]
//! struct User {
//!     first_name: String,
//!     last_name: String,
//!     password_hash: String,
//! }
//!
//! # fn hash_password(value: &str) -> String { format!("hashed:{value}") }
//! impl HasAccessors for User {
//!     fn get_attribute(&self, key: &str) -> Option<AttributeValue> {
//!         match key {
//!             "full_name" => Some(AttributeValue::String(self.get_full_name())),
//!             _ => None,
//!         }
//!     }
//! }
//!
//! impl HasMutators for User {}
//!
//! impl User {
//!     fn get_full_name(&self) -> String {
//!         format!("{} {}", self.first_name, self.last_name)
//!     }
//!
//!     fn set_password(&mut self, value: String) {
//!         self.password_hash = hash_password(&value);
//!     }
//! }
//!
//! # async fn example() {
//! let user = User {
//!     first_name: "John".to_string(),
//!     last_name: "Doe".to_string(),
//!     password_hash: String::new(),
//! };
//!
//! // Accessor usage
//! let full_name = user.get_full_name(); // "John Doe"
//!
//! // Mutator usage
//! let mut user = user;
//! user.set_password("secret123".to_string());
//! # }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Accessor/Mutator errors
#[derive(Error, Debug)]
pub enum AttributeError {
    #[error("Invalid attribute value: {0}")]
    InvalidValue(String),

    #[error("Attribute not found: {0}")]
    NotFound(String),

    #[error("Type conversion error: {0}")]
    ConversionError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type AttributeResult<T> = Result<T, AttributeError>;

/// Trait for models with accessor methods
pub trait HasAccessors {
    /// Get a computed/virtual attribute
    fn get_attribute(&self, _key: &str) -> Option<AttributeValue> {
        None
    }

    /// Check if an accessor exists
    fn has_accessor(&self, _key: &str) -> bool {
        false
    }
}

/// Trait for models with mutator methods
pub trait HasMutators {
    /// Set an attribute with transformation
    fn set_attribute(&mut self, _key: &str, _value: AttributeValue) -> AttributeResult<()> {
        Ok(())
    }

    /// Check if a mutator exists
    fn has_mutator(&self, _key: &str) -> bool {
        false
    }
}

/// Represents a value that can be accessed or mutated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
    Json(serde_json::Value),
    Null,
}

impl AttributeValue {
    /// Convert to string or error
    pub fn as_string(&self) -> AttributeResult<String> {
        match self {
            AttributeValue::String(s) => Ok(s.clone()),
            _ => Err(AttributeError::ConversionError(
                "Expected string".to_string(),
            )),
        }
    }

    /// Convert to integer or error
    pub fn as_integer(&self) -> AttributeResult<i64> {
        match self {
            AttributeValue::Integer(i) => Ok(*i),
            _ => Err(AttributeError::ConversionError(
                "Expected integer".to_string(),
            )),
        }
    }

    /// Convert to float or error
    pub fn as_float(&self) -> AttributeResult<f64> {
        match self {
            AttributeValue::Float(f) => Ok(*f),
            _ => Err(AttributeError::ConversionError(
                "Expected float".to_string(),
            )),
        }
    }

    /// Convert to boolean or error
    pub fn as_boolean(&self) -> AttributeResult<bool> {
        match self {
            AttributeValue::Boolean(b) => Ok(*b),
            _ => Err(AttributeError::ConversionError(
                "Expected boolean".to_string(),
            )),
        }
    }

    /// Convert to DateTime or error
    pub fn as_datetime(&self) -> AttributeResult<DateTime<Utc>> {
        match self {
            AttributeValue::DateTime(dt) => Ok(*dt),
            _ => Err(AttributeError::ConversionError(
                "Expected datetime".to_string(),
            )),
        }
    }

    /// Check if null
    pub fn is_null(&self) -> bool {
        matches!(self, AttributeValue::Null)
    }

    /// Convert to i64 (parses string if needed)
    pub fn as_i64(&self) -> AttributeResult<i64> {
        match self {
            AttributeValue::Integer(i) => Ok(*i),
            AttributeValue::String(s) => s
                .parse::<i64>()
                .map_err(|_| AttributeError::ConversionError("Expected integer".to_string())),
            _ => Err(AttributeError::ConversionError(
                "Expected integer".to_string(),
            )),
        }
    }

    /// Convert to JSON value
    pub fn as_json(&self) -> AttributeResult<serde_json::Value> {
        match self {
            AttributeValue::Json(v) => Ok(v.clone()),
            _ => Err(AttributeError::ConversionError("Expected JSON".to_string())),
        }
    }

    /// Convert to f64
    pub fn as_f64(&self) -> AttributeResult<f64> {
        match self {
            Self::Float(f) => Ok(*f),
            Self::Integer(i) => Ok(*i as f64),
            Self::String(s) => s
                .parse()
                .map_err(|_| AttributeError::ConversionError("Cannot parse to f64".to_string())),
            _ => Err(AttributeError::ConversionError(
                "Cannot convert to f64".to_string(),
            )),
        }
    }

    /// Convert to bool
    pub fn as_bool(&self) -> AttributeResult<bool> {
        match self {
            Self::Boolean(b) => Ok(*b),
            Self::Integer(i) => Ok(*i != 0),
            Self::String(s) => Ok(s == "true" || s == "1"),
            _ => Err(AttributeError::ConversionError(
                "Cannot convert to bool".to_string(),
            )),
        }
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for AttributeValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<f64> for AttributeValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<serde_json::Value> for AttributeValue {
    fn from(v: serde_json::Value) -> Self {
        Self::Json(v)
    }
}

/// Storage for virtual/computed attributes
#[derive(Debug, Clone, Default)]
pub struct AttributeBag {
    attributes: HashMap<String, AttributeValue>,
}

impl AttributeBag {
    /// Create a new attribute bag
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    /// Set an attribute
    pub fn set(&mut self, key: impl Into<String>, value: AttributeValue) {
        self.attributes.insert(key.into(), value);
    }

    /// Get an attribute
    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    /// Remove an attribute
    pub fn remove(&mut self, key: &str) -> Option<AttributeValue> {
        self.attributes.remove(key)
    }

    /// Check if attribute exists
    pub fn has(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get all attribute keys
    pub fn keys(&self) -> Vec<&String> {
        self.attributes.keys().collect()
    }

    /// Clear all attributes
    pub fn clear(&mut self) {
        self.attributes.clear();
    }

    /// Get number of attributes
    pub fn len(&self) -> usize {
        self.attributes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }
}

/// Common accessor implementations
pub mod common_accessors {
    use super::*;

    /// Uppercase accessor
    pub fn uppercase(value: &str) -> String {
        value.to_uppercase()
    }

    /// Lowercase accessor
    pub fn lowercase(value: &str) -> String {
        value.to_lowercase()
    }

    /// Title case accessor
    pub fn title_case(value: &str) -> String {
        value
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Truncate accessor
    pub fn truncate(value: &str, length: usize) -> String {
        if value.chars().count() <= length {
            value.to_string()
        } else {
            let truncated: String = value.chars().take(length).collect();
            format!("{}...", truncated)
        }
    }

    /// Strip HTML accessor
    pub fn strip_html(value: &str) -> String {
        // Simple HTML stripping (use a proper library in production)
        let re = regex::Regex::new(r"<[^>]*>").unwrap();
        re.replace_all(value, "").to_string()
    }
}

/// Common mutator implementations
pub mod common_mutators {
    use super::*;

    /// Trim mutator
    pub fn trim(value: String) -> String {
        value.trim().to_string()
    }

    /// Hash password mutator
    pub fn hash_password(value: &str) -> String {
        // Use bcrypt for password hashing
        bcrypt::hash(value, bcrypt::DEFAULT_COST).unwrap_or_default()
    }

    /// Encrypt mutator
    pub fn encrypt(value: &str) -> String {
        // Simple base64 encoding (use proper encryption in production)
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(value)
    }

    /// Decrypt accessor
    pub fn decrypt(value: &str) -> AttributeResult<String> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(value)
            .map_err(|e| AttributeError::ConversionError(e.to_string()))
            .and_then(|bytes| {
                String::from_utf8(bytes).map_err(|e| AttributeError::ConversionError(e.to_string()))
            })
    }

    /// Slugify mutator
    pub fn slugify(value: &str) -> String {
        value
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_value_conversions() {
        let str_val = AttributeValue::String("42".to_string());
        assert_eq!(str_val.as_i64().unwrap(), 42);
        assert_eq!(str_val.as_string().unwrap(), "42");

        let int_val = AttributeValue::Integer(100);
        assert_eq!(int_val.as_i64().unwrap(), 100);
        assert_eq!(int_val.as_f64().unwrap(), 100.0);

        let bool_val = AttributeValue::Boolean(true);
        assert_eq!(bool_val.as_bool().unwrap(), true);

        let null_val = AttributeValue::Null;
        assert!(null_val.is_null());
    }

    #[test]
    fn test_attribute_bag() {
        let mut bag = AttributeBag::new();

        bag.set("name", AttributeValue::String("John".to_string()));
        bag.set("age", AttributeValue::Integer(30));

        assert!(bag.has("name"));
        assert!(bag.has("age"));
        assert_eq!(bag.len(), 2);

        assert_eq!(bag.get("name").unwrap().as_string().unwrap(), "John");
        assert_eq!(bag.get("age").unwrap().as_i64().unwrap(), 30);

        bag.remove("name");
        assert!(!bag.has("name"));
        assert_eq!(bag.len(), 1);

        bag.clear();
        assert!(bag.is_empty());
    }

    #[test]
    fn test_common_accessors() {
        assert_eq!(common_accessors::uppercase("hello"), "HELLO");
        assert_eq!(common_accessors::lowercase("HELLO"), "hello");
        assert_eq!(common_accessors::title_case("hello world"), "Hello World");
        assert_eq!(common_accessors::truncate("hello world", 5), "hello...");
        assert_eq!(common_accessors::strip_html("<p>Hello</p>"), "Hello");
    }

    #[test]
    fn test_common_mutators() {
        assert_eq!(common_mutators::trim("  hello  ".to_string()), "hello");
        assert_eq!(common_mutators::slugify("Hello World!"), "hello-world");

        let encrypted = common_mutators::encrypt("secret");
        let decrypted = common_mutators::decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, "secret");
    }
}
