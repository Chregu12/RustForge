//! Password hashing and verification
//!
//! Provides secure password hashing using bcrypt or argon2 algorithms.

use crate::error::{AuthError, AuthResult};

/// Password hashing algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// Bcrypt (recommended default)
    Bcrypt,
    /// Argon2 (more resistant to GPU/ASIC attacks)
    Argon2,
}

/// Password hasher for secure password storage
///
/// # Examples
///
/// ```
/// use rf_auth::PasswordHasher;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Create hasher with bcrypt (cost = 12)
/// let hasher = PasswordHasher::bcrypt(12)?;
///
/// // Hash password
/// let password = "my_secure_password";
/// let hash = hasher.hash(password)?;
///
/// // Verify password
/// assert!(hasher.verify(password, &hash)?);
/// assert!(!hasher.verify("wrong_password", &hash)?);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PasswordHasher {
    algorithm: HashAlgorithm,
    bcrypt_cost: u32,
}

impl PasswordHasher {
    /// Create a new password hasher with bcrypt
    ///
    /// # Arguments
    ///
    /// * `cost` - Bcrypt cost factor (4-31, recommended: 12)
    ///
    /// # Errors
    ///
    /// Returns `AuthError::InvalidBcryptCost` if cost is not in range 4-31
    pub fn bcrypt(cost: u32) -> AuthResult<Self> {
        if !(4..=31).contains(&cost) {
            return Err(AuthError::InvalidBcryptCost);
        }

        Ok(Self {
            algorithm: HashAlgorithm::Bcrypt,
            bcrypt_cost: cost,
        })
    }

    /// Create a new password hasher with argon2
    pub fn argon2() -> AuthResult<Self> {
        Ok(Self {
            algorithm: HashAlgorithm::Argon2,
            bcrypt_cost: 0, // Not used for argon2
        })
    }

    /// Hash a password
    ///
    /// # Arguments
    ///
    /// * `password` - Plain text password to hash
    ///
    /// # Returns
    ///
    /// Hashed password string that can be stored in database
    ///
    /// # Errors
    ///
    /// Returns `AuthError::HashingFailed` if hashing fails
    pub fn hash(&self, password: &str) -> AuthResult<String> {
        match self.algorithm {
            HashAlgorithm::Bcrypt => {
                let hash = bcrypt::hash(password, self.bcrypt_cost)
                    .map_err(|e| AuthError::HashingFailed {
                        source: anyhow::anyhow!("Bcrypt hashing failed: {}", e),
                    })?;
                Ok(hash)
            }
            HashAlgorithm::Argon2 => {
                use argon2::{
                    password_hash::{PasswordHasher as _, SaltString},
                    Argon2,
                };
                use rand_core::OsRng;

                let salt = SaltString::generate(&mut OsRng);
                let argon2 = Argon2::default();

                let hash = argon2
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|e| AuthError::HashingFailed {
                        source: anyhow::anyhow!("Argon2 hashing failed: {}", e),
                    })?
                    .to_string();

                Ok(hash)
            }
        }
    }

    /// Verify a password against a hash
    ///
    /// # Arguments
    ///
    /// * `password` - Plain text password to verify
    /// * `hash` - Hash to verify against
    ///
    /// # Returns
    ///
    /// `true` if password matches hash, `false` otherwise
    ///
    /// # Errors
    ///
    /// Returns `AuthError::HashingFailed` if verification fails
    pub fn verify(&self, password: &str, hash: &str) -> AuthResult<bool> {
        // Auto-detect algorithm from hash format
        if hash.starts_with("$2") {
            // Bcrypt hash
            let is_valid = bcrypt::verify(password, hash).map_err(|e| {
                AuthError::HashingFailed {
                    source: anyhow::anyhow!("Bcrypt verification failed: {}", e),
                }
            })?;
            Ok(is_valid)
        } else if hash.starts_with("$argon2") {
            // Argon2 hash
            use argon2::{
                password_hash::{PasswordHash, PasswordVerifier},
                Argon2,
            };

            let parsed_hash = PasswordHash::new(hash).map_err(|e| {
                AuthError::HashingFailed {
                    source: anyhow::anyhow!("Invalid argon2 hash: {}", e),
                }
            })?;

            let argon2 = Argon2::default();
            let is_valid = argon2
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok();

            Ok(is_valid)
        } else {
            Err(AuthError::HashingFailed {
                source: anyhow::anyhow!("Unknown hash format"),
            })
        }
    }

    /// Verify a password with timing-safe comparison
    ///
    /// This method performs verification in constant time to prevent
    /// timing attacks.
    ///
    /// # Arguments
    ///
    /// * `password` - Plain text password to verify
    /// * `hash` - Hash to verify against
    ///
    /// # Returns
    ///
    /// `true` if password matches hash, `false` otherwise
    ///
    /// # Errors
    ///
    /// Returns `AuthError::HashingFailed` if verification fails
    pub fn verify_timing_safe(&self, password: &str, hash: &str) -> AuthResult<bool> {
        // First verify normally
        let is_valid = self.verify(password, hash)?;

        // The actual hash verification in bcrypt/argon2 is already timing-safe
        // This method exists for API consistency and future enhancements
        Ok(is_valid)
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::bcrypt(12).expect("Default bcrypt cost is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcrypt_hashing() {
        let hasher = PasswordHasher::bcrypt(4).unwrap(); // Low cost for tests
        let password = "test_password_123";

        let hash = hasher.hash(password).unwrap();
        assert!(hash.starts_with("$2"));
        assert!(hasher.verify(password, &hash).unwrap());
        assert!(!hasher.verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_argon2_hashing() {
        let hasher = PasswordHasher::argon2().unwrap();
        let password = "test_password_123";

        let hash = hasher.hash(password).unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(hasher.verify(password, &hash).unwrap());
        assert!(!hasher.verify("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_bcrypt_auto_detection() {
        let bcrypt_hasher = PasswordHasher::bcrypt(4).unwrap();
        let argon2_hasher = PasswordHasher::argon2().unwrap();

        let password = "test_password";
        let bcrypt_hash = bcrypt_hasher.hash(password).unwrap();

        // Should work with argon2 hasher too (auto-detection)
        assert!(argon2_hasher.verify(password, &bcrypt_hash).unwrap());
    }

    #[test]
    fn test_timing_safe_verify() {
        let hasher = PasswordHasher::bcrypt(4).unwrap();
        let password = "test_password";
        let hash = hasher.hash(password).unwrap();

        assert!(hasher.verify_timing_safe(password, &hash).unwrap());
        assert!(!hasher.verify_timing_safe("wrong", &hash).unwrap());
    }

    #[test]
    fn test_invalid_bcrypt_cost() {
        assert!(PasswordHasher::bcrypt(3).is_err());
        assert!(PasswordHasher::bcrypt(32).is_err());
        assert!(PasswordHasher::bcrypt(12).is_ok());
    }

    #[test]
    fn test_default_hasher() {
        let hasher = PasswordHasher::default();
        let password = "test_password";
        let hash = hasher.hash(password).unwrap();
        assert!(hasher.verify(password, &hash).unwrap());
    }
}
