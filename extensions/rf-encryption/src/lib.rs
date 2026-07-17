//! Encryption service for RustForge
//!
//! Provides AES-256-GCM encryption and decryption, similar to Laravel's encryption.
//!
//! # Quick Start
//!
//! ```rust
//! use rf_encryption::{Encryptor, Cipher};
//!
//! # fn example() -> rf_encryption::Result<()> {
//! let encryptor = Encryptor::new()
//!     .key("base64:your-32-byte-key-here-encoded==")
//!     .cipher(Cipher::Aes256Gcm);
//!
//! let encrypted = encryptor.encrypt("secret data")?;
//! let decrypted = encryptor.decrypt(&encrypted)?;
//! assert_eq!(decrypted, "secret data");
//! # Ok(())
//! # }
//! ```

mod casting;
mod encryptor;

pub use casting::{Encrypted, EncryptedField};
pub use encryptor::{Cipher, EncryptionError, Encryptor};

pub type Result<T> = std::result::Result<T, EncryptionError>;
