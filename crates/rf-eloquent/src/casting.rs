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
//!     published_at: chrono::DateTime<chrono::Utc>,
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
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
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

/// Process-global encryption key used by [`CastType::Encrypted`].
///
/// When unset, the resolver falls back to the `APP_KEY` / `RF_APP_KEY`
/// environment variables (the RustForge / Laravel convention).
static ENCRYPTION_KEY: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

/// Configure the encryption key used for `CastType::Encrypted` attributes.
///
/// Accepts a base64-encoded 32-byte key, optionally prefixed with `base64:`
/// (produced by `rf_encryption::Encryptor::generate_key`). This is the key
/// used to AES-256-GCM encrypt attribute values at rest.
///
/// If no key is configured here, the `APP_KEY` (then `RF_APP_KEY`) environment
/// variable is used instead.
pub fn set_encryption_key(key: impl Into<String>) {
    if let Ok(mut guard) = ENCRYPTION_KEY.write() {
        *guard = Some(key.into());
    }
}

/// Build the encryptor for `CastType::Encrypted` from the configured key.
///
/// Errors (rather than silently degrading to a reversible encoding) when no
/// key is available, so encrypted attributes are never stored in the clear.
fn resolve_encryptor() -> CastResult<rf_encryption::Encryptor> {
    let key = ENCRYPTION_KEY
        .read()
        .ok()
        .and_then(|guard| guard.clone())
        .or_else(|| std::env::var("APP_KEY").ok())
        .or_else(|| std::env::var("RF_APP_KEY").ok())
        .ok_or_else(|| {
            CastError::EncryptionError(
                "No encryption key configured for CastType::Encrypted. Set the APP_KEY \
                 environment variable (base64) or call rf_eloquent::set_encryption_key(...)"
                    .to_string(),
            )
        })?;

    rf_encryption::Encryptor::new()
        .key(key)
        .build()
        .map_err(|e| CastError::EncryptionError(e.to_string()))
}

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

/// Trait for custom attribute casters
///
/// Implement this trait to create a custom caster that can be registered
/// via `register_caster()` and used with `CastType::Custom("name")`.
///
/// # Example
///
/// ```rust,no_run
/// use rf_eloquent::casting::{Castable, CastedValue, CastResult, register_caster};
///
/// struct PointCaster;
///
/// impl Castable for PointCaster {
///     fn get(&self, value: &str) -> CastResult<CastedValue> {
///         // Parse "x,y" format
///         let parts: Vec<&str> = value.split(',').collect();
///         let json = serde_json::json!({
///             "x": parts.get(0).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
///             "y": parts.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
///         });
///         Ok(CastedValue::Json(json))
///     }
///
///     fn set(&self, value: CastedValue) -> CastResult<String> {
///         if let CastedValue::Json(j) = value {
///             Ok(format!("{},{}", j["x"], j["y"]))
///         } else {
///             Ok(String::new())
///         }
///     }
/// }
///
/// // Register once at application startup
/// register_caster("point", PointCaster);
/// ```
pub trait Castable: Send + Sync {
    /// Cast a database string value to a typed `CastedValue`
    fn get(&self, value: &str) -> CastResult<CastedValue>;

    /// Convert a `CastedValue` back to a database string
    fn set(&self, value: CastedValue) -> CastResult<String>;
}

/// Registry for custom attribute casters
pub struct CustomCasterRegistry {
    casters: HashMap<&'static str, Arc<dyn Castable>>,
}

impl CustomCasterRegistry {
    fn new() -> Self {
        Self {
            casters: HashMap::new(),
        }
    }

    /// Register a custom caster under the given name
    pub fn register(&mut self, name: &'static str, caster: impl Castable + 'static) {
        self.casters.insert(name, Arc::new(caster));
    }

    /// Look up a caster by name
    pub fn get_caster(&self, name: &str) -> Option<Arc<dyn Castable>> {
        self.casters.get(name).cloned()
    }
}

/// Global custom caster registry (thread-safe)
static CUSTOM_CASTERS: Lazy<RwLock<CustomCasterRegistry>> =
    Lazy::new(|| RwLock::new(CustomCasterRegistry::new()));

/// Register a custom attribute caster globally
///
/// Call this once at application startup before any casting takes place.
///
/// # Example
///
/// ```rust,no_run
/// use rf_eloquent::casting::{Castable, CastedValue, CastResult, register_caster};
///
/// struct MoneyCaster;
///
/// impl Castable for MoneyCaster {
///     fn get(&self, value: &str) -> CastResult<CastedValue> {
///         let cents: i64 = value.parse().map_err(|e: std::num::ParseIntError| {
///             rf_eloquent::casting::CastError::CastFailed(e.to_string())
///         })?;
///         Ok(CastedValue::Float(cents as f64 / 100.0))
///     }
///
///     fn set(&self, value: CastedValue) -> CastResult<String> {
///         if let CastedValue::Float(f) = value {
///             Ok(((f * 100.0).round() as i64).to_string())
///         } else {
///             Ok("0".to_string())
///         }
///     }
/// }
///
/// register_caster("money", MoneyCaster);
/// ```
pub fn register_caster(name: &'static str, caster: impl Castable + 'static) {
    if let Ok(mut registry) = CUSTOM_CASTERS.write() {
        registry.register(name, caster);
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
            // AES-256-GCM decryption using the configured application key.
            let plaintext = resolve_encryptor()?
                .decrypt(value)
                .map_err(|e| CastError::DecryptionError(e.to_string()))?;
            Ok(CastedValue::String(plaintext))
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
        CastType::Custom(name) => {
            if let Ok(registry) = CUSTOM_CASTERS.read() {
                if let Some(caster) = registry.get_caster(name) {
                    caster.get(value)
                } else {
                    Err(CastError::CastFailed(format!(
                        "Custom caster '{}' not found. Register it with register_caster()",
                        name
                    )))
                }
            } else {
                Err(CastError::CastFailed(
                    "Custom caster registry lock poisoned".to_string(),
                ))
            }
        }
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
            // AES-256-GCM encryption using the configured application key.
            resolve_encryptor()?
                .encrypt(&s)
                .map_err(|e| CastError::EncryptionError(e.to_string()))
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
        (value, CastType::Custom(name)) => {
            if let Ok(registry) = CUSTOM_CASTERS.read() {
                if let Some(caster) = registry.get_caster(name) {
                    caster.set(value)
                } else {
                    Err(CastError::CastFailed(format!(
                        "Custom caster '{}' not found",
                        name
                    )))
                }
            } else {
                Err(CastError::CastFailed(
                    "Custom caster registry lock poisoned".to_string(),
                ))
            }
        }
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

    #[test]
    fn test_encrypted_cast_uses_real_aes_gcm() {
        // Configure a real 32-byte AES-256 key.
        set_encryption_key(rf_encryption::Encryptor::generate_key());

        let plaintext = "top-secret-value";

        // Encrypt via the Encrypted cast (uncast = value -> DB representation).
        let ciphertext = uncast_value(
            CastedValue::String(plaintext.to_string()),
            CastType::Encrypted,
        )
        .unwrap();

        // Real encryption must NOT be a reversible plaintext encoding:
        // base64 of the plaintext would be recoverable; AES-GCM ciphertext must not be.
        use base64::Engine;
        let naive_b64 = base64::engine::general_purpose::STANDARD.encode(plaintext.as_bytes());
        assert_ne!(
            ciphertext, naive_b64,
            "Encrypted cast must produce ciphertext, not base64 of the plaintext"
        );
        assert!(!ciphertext.contains(plaintext));
        // Decoding the stored value as base64 must not reveal the plaintext.
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&ciphertext) {
            assert_ne!(decoded, plaintext.as_bytes());
        }

        // GCM is nonce-randomized: encrypting again yields different ciphertext.
        let ciphertext2 = uncast_value(
            CastedValue::String(plaintext.to_string()),
            CastType::Encrypted,
        )
        .unwrap();
        assert_ne!(
            ciphertext, ciphertext2,
            "AES-GCM must use a random nonce per encryption"
        );

        // Round-trips back to the original plaintext via the Encrypted cast.
        let recovered = cast_value(&ciphertext, CastType::Encrypted).unwrap();
        assert_eq!(recovered.as_string().unwrap(), plaintext);
    }
}
