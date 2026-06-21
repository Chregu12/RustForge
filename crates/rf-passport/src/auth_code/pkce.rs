//! PKCE (Proof Key for Code Exchange) implementation

use crate::errors::{PassportError, PassportResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

/// PKCE code challenge method
#[derive(Debug, Clone, PartialEq)]
pub enum CodeChallengeMethod {
    /// Plain text (not recommended)
    Plain,
    /// SHA256 hash
    S256,
}

impl CodeChallengeMethod {
    /// Parse from string
    // Intentional inherent `from_str` (fallible, custom error type); keep the name.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> PassportResult<Self> {
        match s {
            "plain" => Ok(Self::Plain),
            "S256" => Ok(Self::S256),
            _ => Err(PassportError::InvalidRequest(format!(
                "Invalid code challenge method: {}",
                s
            ))),
        }
    }

    /// Convert to string
    // Intentional inherent `to_string` for this small enum; keep as-is.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            Self::Plain => "plain".to_string(),
            Self::S256 => "S256".to_string(),
        }
    }
}

/// Generate a code challenge from a code verifier
pub fn generate_code_challenge(
    code_verifier: &str,
    method: &CodeChallengeMethod,
) -> PassportResult<String> {
    // Validate code verifier length (43-128 characters)
    if code_verifier.len() < 43 || code_verifier.len() > 128 {
        return Err(PassportError::InvalidRequest(
            "Code verifier must be between 43 and 128 characters".to_string(),
        ));
    }

    // Validate code verifier characters (unreserved characters only)
    if !code_verifier
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~')
    {
        return Err(PassportError::InvalidRequest(
            "Code verifier contains invalid characters".to_string(),
        ));
    }

    match method {
        CodeChallengeMethod::Plain => Ok(code_verifier.to_string()),
        CodeChallengeMethod::S256 => {
            let mut hasher = Sha256::new();
            hasher.update(code_verifier.as_bytes());
            let hash = hasher.finalize();
            Ok(URL_SAFE_NO_PAD.encode(hash))
        }
    }
}

/// Verify a code verifier against a code challenge
pub fn verify_code_challenge(
    code_verifier: &str,
    code_challenge: &str,
    method: &CodeChallengeMethod,
) -> PassportResult<bool> {
    let computed_challenge = generate_code_challenge(code_verifier, method)?;
    Ok(computed_challenge == code_challenge)
}

/// Generate a random code verifier
pub fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..128)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 128);
        assert!(verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'));
    }

    #[test]
    fn test_plain_challenge() {
        let verifier = "test-verifier-abc123-xyz789-test-verifier-abc1";
        let challenge =
            generate_code_challenge(verifier, &CodeChallengeMethod::Plain).unwrap();
        assert_eq!(challenge, verifier);

        let valid = verify_code_challenge(verifier, &challenge, &CodeChallengeMethod::Plain)
            .unwrap();
        assert!(valid);
    }

    #[test]
    fn test_s256_challenge() {
        let verifier = "test-verifier-abc123-xyz789-test-verifier-abc1";
        let challenge = generate_code_challenge(verifier, &CodeChallengeMethod::S256).unwrap();

        // Challenge should be different from verifier
        assert_ne!(challenge, verifier);

        // Verify should succeed
        let valid =
            verify_code_challenge(verifier, &challenge, &CodeChallengeMethod::S256).unwrap();
        assert!(valid);

        // Wrong verifier should fail (must be 43+ chars)
        let wrong_verifier = "wrong-verifier-abc123-xyz789-wrong-verifier";
        let invalid =
            verify_code_challenge(wrong_verifier, &challenge, &CodeChallengeMethod::S256)
                .unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_invalid_verifier_length() {
        // Too short
        let result = generate_code_challenge("short", &CodeChallengeMethod::Plain);
        assert!(result.is_err());

        // Too long
        let long_verifier = "a".repeat(129);
        let result = generate_code_challenge(&long_verifier, &CodeChallengeMethod::Plain);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_verifier_characters() {
        let invalid_verifier =
            "test-verifier-abc123-xyz789-test-verifier-abc1-invalid!@#$";
        let result = generate_code_challenge(invalid_verifier, &CodeChallengeMethod::Plain);
        assert!(result.is_err());
    }

    #[test]
    fn test_code_challenge_method_from_str() {
        assert_eq!(
            CodeChallengeMethod::from_str("plain").unwrap(),
            CodeChallengeMethod::Plain
        );
        assert_eq!(
            CodeChallengeMethod::from_str("S256").unwrap(),
            CodeChallengeMethod::S256
        );
        assert!(CodeChallengeMethod::from_str("invalid").is_err());
    }
}
