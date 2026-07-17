//! Encryption implementation

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Invalid payload format")]
    InvalidPayload,
}

pub type Result<T> = std::result::Result<T, EncryptionError>;

/// Encryption cipher
#[derive(Debug, Clone, Copy)]
pub enum Cipher {
    /// AES-256-GCM
    Aes256Gcm,
}

/// Encryptor for encrypting and decrypting data
pub struct Encryptor {
    key: Vec<u8>,
    cipher: Cipher,
}

impl Encryptor {
    /// Create a new encryptor builder
    #[allow(clippy::new_ret_no_self)] // Builder pattern: Encryptor::new() intentionally returns EncryptorBuilder
    pub fn new() -> EncryptorBuilder {
        EncryptorBuilder::default()
    }

    /// Generate a new random encryption key
    pub fn generate_key() -> String {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        format!("base64:{}", base64::encode(&key))
    }

    /// Encrypt a string
    pub fn encrypt(&self, data: &str) -> Result<String> {
        self.encrypt_bytes(data.as_bytes())
    }

    /// Encrypt bytes
    pub fn encrypt_bytes(&self, data: &[u8]) -> Result<String> {
        match self.cipher {
            Cipher::Aes256Gcm => self.encrypt_aes256gcm(data),
        }
    }

    /// Decrypt a string
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        let bytes = self.decrypt_bytes(encrypted)?;
        String::from_utf8(bytes)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid UTF-8: {}", e)))
    }

    /// Decrypt bytes
    pub fn decrypt_bytes(&self, encrypted: &str) -> Result<Vec<u8>> {
        match self.cipher {
            Cipher::Aes256Gcm => self.decrypt_aes256gcm(encrypted),
        }
    }

    fn encrypt_aes256gcm(&self, data: &[u8]) -> Result<String> {
        if self.key.len() != 32 {
            return Err(EncryptionError::InvalidKey(
                "AES-256 requires a 32-byte key".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(&nonce, data)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Combine nonce + ciphertext
        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);

        Ok(base64::encode(&payload))
    }

    fn decrypt_aes256gcm(&self, encrypted: &str) -> Result<Vec<u8>> {
        if self.key.len() != 32 {
            return Err(EncryptionError::InvalidKey(
                "AES-256 requires a 32-byte key".to_string(),
            ));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // Decode base64
        let payload = base64::decode(encrypted)?;

        if payload.len() < 12 {
            return Err(EncryptionError::InvalidPayload);
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = payload.split_at(12);
        let mut nonce_array = [0u8; 12];
        nonce_array.copy_from_slice(nonce_bytes);
        let nonce = Nonce::from(nonce_array);

        // Decrypt
        cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
    }
}

impl Default for Encryptor {
    fn default() -> Self {
        Self {
            key: vec![0u8; 32],
            cipher: Cipher::Aes256Gcm,
        }
    }
}

/// Builder for encryptor
pub struct EncryptorBuilder {
    key: Option<Vec<u8>>,
    cipher: Cipher,
}

impl EncryptorBuilder {
    pub fn key(mut self, key: impl AsRef<str>) -> Self {
        let key_str = key.as_ref();

        // Accept "base64:<data>" prefix or plain base64 string; never treat raw input as key bytes
        let decoded = if let Some(stripped) = key_str.strip_prefix("base64:") {
            base64::decode(stripped).ok()
        } else {
            base64::decode(key_str).ok()
        };

        self.key = decoded;
        self
    }

    pub fn cipher(mut self, cipher: Cipher) -> Self {
        self.cipher = cipher;
        self
    }

    pub fn build(self) -> Result<Encryptor> {
        let key = self.key.ok_or_else(|| {
            EncryptionError::InvalidKey(
                "No encryption key provided — call .key() before .build()".to_string(),
            )
        })?;
        Ok(Encryptor {
            key,
            cipher: self.cipher,
        })
    }
}

impl Default for EncryptorBuilder {
    fn default() -> Self {
        Self {
            key: None,
            cipher: Cipher::Aes256Gcm,
        }
    }
}

// Convenience methods that return the builder as the encryptor
impl EncryptorBuilder {
    pub fn encrypt(&self, data: &str) -> Result<String> {
        self.clone().build()?.encrypt(data)
    }

    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        self.clone().build()?.decrypt(encrypted)
    }
}

impl Clone for EncryptorBuilder {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            cipher: self.cipher,
        }
    }
}

mod base64 {
    pub use base64::engine::general_purpose::STANDARD;
    pub use base64::DecodeError;
    pub use base64::Engine;

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key() {
        let key = Encryptor::generate_key();
        assert!(key.starts_with("base64:"));
        assert!(key.len() > 10);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().unwrap();

        let plaintext = "Hello, World!";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_ne!(plaintext, encrypted);
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().unwrap();

        let plaintext = "Test";
        let encrypted1 = encryptor.encrypt(plaintext).unwrap();
        let encrypted2 = encryptor.encrypt(plaintext).unwrap();

        // Should be different due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same value
        assert_eq!(encryptor.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(encryptor.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_invalid() {
        let key = Encryptor::generate_key();
        let encryptor = Encryptor::new().key(&key).build().unwrap();

        let result = encryptor.decrypt("invalid-data");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_key() {
        let key1 = Encryptor::generate_key();
        let key2 = Encryptor::generate_key();

        let encryptor1 = Encryptor::new().key(&key1).build().unwrap();
        let encryptor2 = Encryptor::new().key(&key2).build().unwrap();

        let encrypted = encryptor1.encrypt("secret").unwrap();
        let result = encryptor2.decrypt(&encrypted);

        assert!(result.is_err());
    }
}
