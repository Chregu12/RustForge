//! Password hashing utilities using bcrypt and argon2.
//!
//! This module provides a Laravel-style Hash facade for password hashing
//! and verification.

use bcrypt::{hash as bcrypt_hash, verify as bcrypt_verify, DEFAULT_COST};
use argon2::{
    Argon2,
    PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{rand_core::OsRng, SaltString},
};

/// Hash algorithm to use
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HashAlgorithm {
    /// BCrypt hashing (default)
    #[default]
    Bcrypt,
    /// Argon2 hashing (recommended for new applications)
    Argon2,
}

/// The Hash facade providing password hashing utilities.
///
/// # Examples
///
/// ```rust
/// use rf_global_helpers::Hash;
///
/// // Hash a password
/// let hash = Hash::make("my-password");
///
/// // Verify a password
/// assert!(Hash::check("my-password", &hash));
/// assert!(!Hash::check("wrong-password", &hash));
/// ```
pub struct Hash;

impl Hash {
    /// Hash a password using the default algorithm (BCrypt).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make("my-password");
    /// assert!(!hash.is_empty());
    /// ```
    pub fn make(value: &str) -> String {
        Self::make_with(value, HashAlgorithm::default())
    }

    /// Hash a password using a specific algorithm.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::{Hash, HashAlgorithm};
    ///
    /// let bcrypt_hash = Hash::make_with("password", HashAlgorithm::Bcrypt);
    /// let argon2_hash = Hash::make_with("password", HashAlgorithm::Argon2);
    /// ```
    pub fn make_with(value: &str, algorithm: HashAlgorithm) -> String {
        match algorithm {
            HashAlgorithm::Bcrypt => Self::make_bcrypt(value, DEFAULT_COST),
            HashAlgorithm::Argon2 => Self::make_argon2(value),
        }
    }

    /// Hash a password using BCrypt with a custom cost.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make_bcrypt("password", 12);
    /// ```
    pub fn make_bcrypt(value: &str, cost: u32) -> String {
        bcrypt_hash(value, cost).expect("Failed to hash password with bcrypt")
    }

    /// Hash a password using Argon2.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make_argon2("password");
    /// ```
    pub fn make_argon2(value: &str) -> String {
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);

        argon2
            .hash_password(value.as_bytes(), &salt)
            .expect("Failed to hash password with argon2")
            .to_string()
    }

    /// Check if a plain text password matches a hash.
    ///
    /// This method automatically detects the hashing algorithm used.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make("password");
    /// assert!(Hash::check("password", &hash));
    /// assert!(!Hash::check("wrong", &hash));
    /// ```
    pub fn check(value: &str, hash: &str) -> bool {
        // Try BCrypt first (more common)
        if let Ok(result) = bcrypt_verify(value, hash) {
            return result;
        }

        // Try Argon2
        if let Ok(parsed_hash) = PasswordHash::new(hash) {
            let argon2 = Argon2::default();
            return argon2
                .verify_password(value.as_bytes(), &parsed_hash)
                .is_ok();
        }

        false
    }

    /// Check if a hash was created with BCrypt.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make_bcrypt("password", 12);
    /// assert!(Hash::is_bcrypt(&hash));
    /// ```
    pub fn is_bcrypt(hash: &str) -> bool {
        hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$")
    }

    /// Check if a hash was created with Argon2.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make_argon2("password");
    /// assert!(Hash::is_argon2(&hash));
    /// ```
    pub fn is_argon2(hash: &str) -> bool {
        hash.starts_with("$argon2")
    }

    /// Check if a hash needs to be rehashed.
    ///
    /// This is useful for upgrading password hashes when you change
    /// the hashing algorithm or cost factor.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make("password");
    /// assert!(!Hash::needs_rehash(&hash));
    /// ```
    pub fn needs_rehash(hash: &str) -> bool {
        // For BCrypt, check if the cost is less than the current default
        if Self::is_bcrypt(hash) {
            if let Some(cost_str) = hash.split('$').nth(2) {
                if let Ok(cost) = cost_str.parse::<u32>() {
                    return cost < DEFAULT_COST;
                }
            }
        }

        // For Argon2, could check parameters here
        // For now, assume no rehash needed
        false
    }

    /// Get information about a hash.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_global_helpers::Hash;
    ///
    /// let hash = Hash::make("password");
    /// let info = Hash::info(&hash);
    /// ```
    pub fn info(hash: &str) -> HashInfo {
        if Self::is_bcrypt(hash) {
            let cost = hash
                .split('$')
                .nth(2)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);

            HashInfo {
                algorithm: HashAlgorithm::Bcrypt,
                cost: Some(cost),
            }
        } else if Self::is_argon2(hash) {
            HashInfo {
                algorithm: HashAlgorithm::Argon2,
                cost: None,
            }
        } else {
            HashInfo {
                algorithm: HashAlgorithm::Bcrypt,
                cost: None,
            }
        }
    }
}

/// Information about a password hash
#[derive(Debug, Clone)]
pub struct HashInfo {
    /// The algorithm used
    pub algorithm: HashAlgorithm,
    /// The cost factor (for BCrypt)
    pub cost: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_make() {
        let hash = Hash::make("password123");
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$2"));
    }

    #[test]
    fn test_hash_check() {
        let hash = Hash::make("my-password");
        assert!(Hash::check("my-password", &hash));
        assert!(!Hash::check("wrong-password", &hash));
    }

    #[test]
    fn test_bcrypt_hash() {
        let hash = Hash::make_bcrypt("test123", DEFAULT_COST);
        assert!(Hash::is_bcrypt(&hash));
        assert!(Hash::check("test123", &hash));
    }

    #[test]
    fn test_argon2_hash() {
        let hash = Hash::make_argon2("test123");
        assert!(Hash::is_argon2(&hash));
        assert!(Hash::check("test123", &hash));
    }

    #[test]
    fn test_make_with_bcrypt() {
        let hash = Hash::make_with("password", HashAlgorithm::Bcrypt);
        assert!(Hash::is_bcrypt(&hash));
    }

    #[test]
    fn test_make_with_argon2() {
        let hash = Hash::make_with("password", HashAlgorithm::Argon2);
        assert!(Hash::is_argon2(&hash));
    }

    #[test]
    fn test_is_bcrypt() {
        let hash = Hash::make_bcrypt("test", DEFAULT_COST);
        assert!(Hash::is_bcrypt(&hash));
        assert!(!Hash::is_argon2(&hash));
    }

    #[test]
    fn test_is_argon2() {
        let hash = Hash::make_argon2("test");
        assert!(Hash::is_argon2(&hash));
        assert!(!Hash::is_bcrypt(&hash));
    }

    #[test]
    fn test_needs_rehash() {
        let hash = Hash::make("password");
        assert!(!Hash::needs_rehash(&hash));
    }

    #[test]
    fn test_hash_info_bcrypt() {
        let hash = Hash::make_bcrypt("password", 12);
        let info = Hash::info(&hash);

        assert_eq!(info.algorithm, HashAlgorithm::Bcrypt);
        assert_eq!(info.cost, Some(12));
    }

    #[test]
    fn test_hash_info_argon2() {
        let hash = Hash::make_argon2("password");
        let info = Hash::info(&hash);

        assert_eq!(info.algorithm, HashAlgorithm::Argon2);
        assert_eq!(info.cost, None);
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let hash1 = Hash::make("password1");
        let hash2 = Hash::make("password2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_same_password_different_hashes() {
        // Due to random salt, same password should produce different hashes
        let hash1 = Hash::make("password");
        let hash2 = Hash::make("password");
        assert_ne!(hash1, hash2);

        // But both should verify correctly
        assert!(Hash::check("password", &hash1));
        assert!(Hash::check("password", &hash2));
    }
}
