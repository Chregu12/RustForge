//! # Attribute Casting System
//!
//! Automatically casts model attributes to specific types when retrieved or set.
//! Supports JSON, dates, encrypted values, and custom casters.
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rf_eloquent::prelude::*;
//!
//! #[derive(Clone, Debug)]
//! struct Post {
//!     id: i64,
//!     title: String,
//!     metadata: serde_json::Value,
//!     published_at: chrono::DateTime<Utc>,
//!     views: i64,
//! }
//!
//! impl HasCasts for Post {
//!     fn casts() -> CastRegistry {
//!         CastRegistry::new()
//!             .cast("metadata", CastType::Json)
//!             .cast("published_at", CastType::DateTime)
//!             .cast("views", CastType::Integer)
//!     }
//! }
//! ```

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Casting errors
#[derive(Error, Debug)]
pub enum CastError {
    #[error("Failed to cast value: {0}")]
    CastFailed(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Invalid date format: {0}")]
    InvalidDate(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption error: {0}")]
    DecryptionError(String),
}

pub type CastResult<T> = Result<T, CastError>;

/// Supported cast types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastType {
    /// Cast to string
    String,
    /// Cast to integer
    Integer,
    /// Cast to float
    Float,
    /// Cast to boolean
    Boolean,
    /// Cast to JSON
    Json,
    /// Cast to DateTime
    DateTime,
    /// Cast to Date (without time)
    Date,
    /// Encrypted string
    Encrypted,
    /// Array of values
    Array,
    /// Collection
    Collection,
    /// Custom cast (requires custom caster)
    Custom(&'static str),
}

/// Trait for models with attribute casting
pub trait HasCasts {
    /// Define casts for model attributes
    fn casts() -> CastRegistry;

    /// Cast an attribute to its defined type
    fn cast_attribute(&self, key: &str, value: &str) -> CastResult<CastedValue> {
        let casts = Self::casts();
        if let Some(cast_type) = casts.get(key) {
            cast_value(value, *cast_type)
        } else {
            Ok(CastedValue::String(value.to_string()))
        }
    }

    /// Uncast an attribute to its database representation
    fn uncast_attribute(&self, key: &str, value: CastedValue) -> CastResult<String> {
        let casts = Self::casts();
        if let Some(cast_type) = casts.get(key) {
            uncast_value(value, *cast_type)
        } else {
            match value {
                CastedValue::String(s) => Ok(s),
                _ => Err(CastError::TypeMismatch {
                    expected: "string".to_string(),
                    actual: format!("{:?}", value),
                }),
            }
        }
    }
}

/// Registry of cast definitions for a model
#[derive(Debug, Clone)]
pub struct CastRegistry {
    casts: HashMap<String, CastType>,
}

impl CastRegistry {
    /// Create a new empty cast registry
    pub fn new() -> Self {
        Self {
            casts: HashMap::new(),
        }
    }

    /// Add a cast definition
    pub fn cast(mut self, attribute: impl Into<String>, cast_type: CastType) -> Self {
        self.casts.insert(attribute.into(), cast_type);
        self
    }

    /// Get the cast type for an attribute
    pub fn get(&self, attribute: &str) -> Option<&CastType> {
        self.casts.get(attribute)
    }

    /// Check if an attribute has a cast
    pub fn has(&self, attribute: &str) -> bool {
        self.casts.contains_key(attribute)
    }

    /// Remove a cast definition
    pub fn remove(&mut self, attribute: &str) -> Option<CastType> {
        self.casts.remove(attribute)
    }

    /// Get all cast definitions
    pub fn all(&self) -> &HashMap<String, CastType> {
        &self.casts
    }
}

impl Default for CastRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a casted value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CastedValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Json(serde_json::Value),
    DateTime(DateTime<Utc>),
    Array(Vec<CastedValue>),
    Null,
}

impl CastedValue {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Convert to string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Convert to i64
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Convert to f64
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Convert to bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

/// Cast a string value to the specified type
pub fn cast_value(value: &str, cast_type: CastType) -> CastResult<CastedValue> {
    match cast_type {
        CastType::String => Ok(CastedValue::String(value.to_string())),
        CastType::Integer => {
            let i = value
                .parse::<i64>()
                .map_err(|e| CastError::CastFailed(e.to_string()))?;
            Ok(CastedValue::Integer(i))
        }
        CastType::Float => {
            let f = value
                .parse::<f64>()
                .map_err(|e| CastError::CastFailed(e.to_string()))?;
            Ok(CastedValue::Float(f))
        }
        CastType::Boolean => {
            let b = match value.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => {
                    return Err(CastError::CastFailed(format!(
                        "Invalid boolean value: {}",
                        value
                    )))
                }
            };
            Ok(CastedValue::Boolean(b))
        }
        CastType::Json => {
            let json = serde_json::from_str(value)?;
            Ok(CastedValue::Json(json))
        }
        CastType::DateTime => {
            let dt = DateTime::parse_from_rfc3339(value)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                    NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                })
                .map_err(|e| CastError::InvalidDate(e.to_string()))?;
            Ok(CastedValue::DateTime(dt))
        }
        CastType::Date => {
            // Parse as date and convert to DateTime at midnight
            let dt =
                NaiveDateTime::parse_from_str(&format!("{} 00:00:00", value), "%Y-%m-%d %H:%M:%S")
                    .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                    .map_err(|e| CastError::InvalidDate(e.to_string()))?;
            Ok(CastedValue::DateTime(dt))
        }
        CastType::Encrypted => {
            // Decrypt the value (simplified - use proper encryption in production)
            use base64::Engine;
            let decrypted = base64::engine::general_purpose::STANDARD
                .decode(value)
                .map_err(|e| CastError::DecryptionError(e.to_string()))?;
            let s = String::from_utf8(decrypted)
                .map_err(|e| CastError::DecryptionError(e.to_string()))?;
            Ok(CastedValue::String(s))
        }
        CastType::Array => {
            let arr: Vec<serde_json::Value> = serde_json::from_str(value)?;
            let casted: Vec<CastedValue> = arr.into_iter().map(CastedValue::Json).collect();
            Ok(CastedValue::Array(casted))
        }
        CastType::Collection => {
            // Similar to array
            let arr: Vec<serde_json::Value> = serde_json::from_str(value)?;
            let casted: Vec<CastedValue> = arr.into_iter().map(CastedValue::Json).collect();
            Ok(CastedValue::Array(casted))
        }
        CastType::Custom(name) => Err(CastError::CastFailed(format!(
            "Custom caster '{}' not implemented",
            name
        ))),
    }
}

/// Uncast a value back to its database string representation
pub fn uncast_value(value: CastedValue, cast_type: CastType) -> CastResult<String> {
    match (value, cast_type) {
        (CastedValue::String(s), CastType::String) => Ok(s),
        (CastedValue::Integer(i), CastType::Integer) => Ok(i.to_string()),
        (CastedValue::Float(f), CastType::Float) => Ok(f.to_string()),
        (CastedValue::Boolean(b), CastType::Boolean) => Ok(b.to_string()),
        (CastedValue::Json(j), CastType::Json) => Ok(j.to_string()),
        (CastedValue::DateTime(dt), CastType::DateTime) => Ok(dt.to_rfc3339()),
        (CastedValue::DateTime(dt), CastType::Date) => Ok(dt.format("%Y-%m-%d").to_string()),
        (CastedValue::String(s), CastType::Encrypted) => {
            // Encrypt the value (simplified)
            use base64::Engine;
            Ok(base64::engine::general_purpose::STANDARD.encode(s.as_bytes()))
        }
        (CastedValue::Array(arr), CastType::Array | CastType::Collection) => {
            let json_arr: Vec<serde_json::Value> = arr
                .into_iter()
                .filter_map(|v| match v {
                    CastedValue::Json(j) => Some(j),
                    _ => None,
                })
                .collect();
            Ok(serde_json::to_string(&json_arr)?)
        }
        (CastedValue::Null, _) => Ok(String::new()),
        (value, cast_type) => Err(CastError::TypeMismatch {
            expected: format!("{:?}", cast_type),
            actual: format!("{:?}", value),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cast_string() {
        let result = cast_value("hello", CastType::String).unwrap();
        assert_eq!(result.as_string().unwrap(), "hello");
    }

    #[test]
    fn test_cast_integer() {
        let result = cast_value("42", CastType::Integer).unwrap();
        assert_eq!(result.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_cast_float() {
        let result = cast_value("3.14", CastType::Float).unwrap();
        assert_eq!(result.as_f64().unwrap(), 3.14);
    }

    #[test]
    fn test_cast_boolean() {
        assert!(cast_value("true", CastType::Boolean)
            .unwrap()
            .as_bool()
            .unwrap());
        assert!(cast_value("1", CastType::Boolean)
            .unwrap()
            .as_bool()
            .unwrap());
        assert!(!cast_value("false", CastType::Boolean)
            .unwrap()
            .as_bool()
            .unwrap());
        assert!(!cast_value("0", CastType::Boolean)
            .unwrap()
            .as_bool()
            .unwrap());
    }

    #[test]
    fn test_cast_json() {
        let result = cast_value(r#"{"key":"value"}"#, CastType::Json).unwrap();
        match result {
            CastedValue::Json(j) => {
                assert_eq!(j["key"], "value");
            }
            _ => panic!("Expected JSON value"),
        }
    }

    #[test]
    fn test_cast_registry() {
        let registry = CastRegistry::new()
            .cast("name", CastType::String)
            .cast("age", CastType::Integer)
            .cast("metadata", CastType::Json);

        assert!(registry.has("name"));
        assert!(registry.has("age"));
        assert!(registry.has("metadata"));
        assert_eq!(*registry.get("name").unwrap(), CastType::String);
        assert_eq!(*registry.get("age").unwrap(), CastType::Integer);
    }

    #[test]
    fn test_uncast_value() {
        let value = CastedValue::Integer(42);
        let result = uncast_value(value, CastType::Integer).unwrap();
        assert_eq!(result, "42");

        let value = CastedValue::Boolean(true);
        let result = uncast_value(value, CastType::Boolean).unwrap();
        assert_eq!(result, "true");
    }
}
