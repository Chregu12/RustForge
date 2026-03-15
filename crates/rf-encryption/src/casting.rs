//! Encrypted field casting support

use crate::{Encryptor, Result};
use serde::{Deserialize, Serialize};

/// Trait for encrypted fields
pub trait EncryptedField {
    /// Encrypt the field value
    fn encrypt(&self, encryptor: &Encryptor) -> Result<String>;

    /// Decrypt the field value
    fn decrypt(encrypted: &str, encryptor: &Encryptor) -> Result<Self>
    where
        Self: Sized;
}

/// Wrapper for encrypted values
#[derive(Debug, Clone)]
pub struct Encrypted<T> {
    value: T,
    encrypted: Option<String>,
}

impl<T> Encrypted<T> {
    /// Create a new encrypted value
    pub fn new(value: T) -> Self {
        Self {
            value,
            encrypted: None,
        }
    }

    /// Get the decrypted value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get the encrypted string (if available)
    pub fn encrypted(&self) -> Option<&str> {
        self.encrypted.as_deref()
    }

    /// Set the encrypted string
    pub fn set_encrypted(&mut self, encrypted: String) {
        self.encrypted = Some(encrypted);
    }
}

impl<T: ToString> Encrypted<T> {
    /// Encrypt the value
    pub fn encrypt(&mut self, encryptor: &Encryptor) -> Result<String> {
        let encrypted = encryptor.encrypt(&self.value.to_string())?;
        self.encrypted = Some(encrypted.clone());
        Ok(encrypted)
    }
}

impl<T: std::str::FromStr> Encrypted<T>
where
    T::Err: std::fmt::Display,
{
    /// Decrypt a value
    pub fn decrypt(encrypted: &str, encryptor: &Encryptor) -> Result<Self> {
        let decrypted = encryptor.decrypt(encrypted)?;
        let value = decrypted.parse::<T>().map_err(|e| {
            crate::EncryptionError::DecryptionFailed(format!(
                "Failed to parse decrypted value: {}",
                e
            ))
        })?;

        Ok(Self {
            value,
            encrypted: Some(encrypted.to_string()),
        })
    }
}

// Implement Serialize/Deserialize for Encrypted<T>
impl<T: Serialize> Serialize for Encrypted<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(ref encrypted) = self.encrypted {
            encrypted.serialize(serializer)
        } else {
            self.value.serialize(serializer)
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Encrypted<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(Self::new(value))
    }
}

/// Macro to mark a field as encrypted
#[macro_export]
macro_rules! encrypted {
    ($field:expr, $encryptor:expr) => {
        $crate::Encrypted::new($field).encrypt($encryptor)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_wrapper() {
        let encrypted = Encrypted::new("secret".to_string());
        assert_eq!(encrypted.value(), "secret");
        assert!(encrypted.encrypted().is_none());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = Encryptor::new().key(Encryptor::generate_key()).build().unwrap();

        let mut encrypted = Encrypted::new("secret".to_string());
        let encrypted_str = encrypted.encrypt(&encryptor).unwrap();

        let decrypted = Encrypted::<String>::decrypt(&encrypted_str, &encryptor).unwrap();
        assert_eq!(decrypted.value(), "secret");
    }

    #[test]
    fn test_encrypted_numbers() {
        let encryptor = Encryptor::new().key(Encryptor::generate_key()).build().unwrap();

        let mut encrypted = Encrypted::new(42);
        let encrypted_str = encrypted.encrypt(&encryptor).unwrap();

        let decrypted = Encrypted::<i32>::decrypt(&encrypted_str, &encryptor).unwrap();
        assert_eq!(*decrypted.value(), 42);
    }

    #[test]
    fn test_serialization() {
        let encrypted = Encrypted::new("test".to_string());
        let json = serde_json::to_string(&encrypted).unwrap();
        assert!(json.contains("test"));

        let deserialized: Encrypted<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.value(), "test");
    }
}
