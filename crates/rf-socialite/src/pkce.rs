//! PKCE (Proof Key for Code Exchange) support for OAuth2
//!
//! PKCE is a security extension to OAuth2 that prevents authorization code
//! interception attacks.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE code verifier and challenge
#[derive(Debug, Clone)]
pub struct Pkce {
    /// Code verifier (random string)
    pub code_verifier: String,
    /// Code challenge (SHA256 hash of verifier)
    pub code_challenge: String,
    /// Challenge method (always S256)
    pub code_challenge_method: String,
}

impl Pkce {
    /// Generate a new PKCE code verifier and challenge
    ///
    /// # Example
    ///
    /// ```
    /// use rf_socialite::pkce::Pkce;
    ///
    /// let pkce = Pkce::generate();
    /// assert_eq!(pkce.code_challenge_method, "S256");
    /// assert!(pkce.code_verifier.len() >= 43);
    /// ```
    pub fn generate() -> Self {
        let code_verifier = Self::generate_verifier();
        let code_challenge = Self::generate_challenge(&code_verifier);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".to_string(),
        }
    }

    /// Generate a random code verifier
    ///
    /// The code verifier is a cryptographically random string between 43-128 characters
    fn generate_verifier() -> String {
        let mut rng = rand::thread_rng();
        let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        URL_SAFE_NO_PAD.encode(&random_bytes)
    }

    /// Generate code challenge from verifier using SHA256
    fn generate_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce() {
        let pkce = Pkce::generate();
        assert!(!pkce.code_verifier.is_empty());
        assert!(!pkce.code_challenge.is_empty());
        assert_eq!(pkce.code_challenge_method, "S256");
    }

    #[test]
    fn test_verifier_length() {
        let pkce = Pkce::generate();
        assert!(pkce.code_verifier.len() >= 43);
        assert!(pkce.code_verifier.len() <= 128);
    }

    #[test]
    fn test_challenge_deterministic() {
        let verifier = "test_verifier_123";
        let challenge1 = Pkce::generate_challenge(verifier);
        let challenge2 = Pkce::generate_challenge(verifier);
        assert_eq!(challenge1, challenge2);
    }

    #[test]
    fn test_different_verifiers_different_challenges() {
        let pkce1 = Pkce::generate();
        let pkce2 = Pkce::generate();
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
    }
}
