//! Signed URLs with expiration support.

use chrono::{DateTime, Duration, Utc};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// A signed URL.
#[derive(Debug, Clone)]
pub struct SignedUrl {
    url: String,
    signature: String,
    expires_at: Option<DateTime<Utc>>,
}

impl SignedUrl {
    /// Create a new signed URL.
    pub fn new(url: impl Into<String>, secret: &str, expires_at: Option<DateTime<Utc>>) -> Self {
        let url = url.into();
        let signature = Self::generate_signature(&url, secret, expires_at.as_ref());

        Self {
            url,
            signature,
            expires_at,
        }
    }

    /// Generate a signature for the URL.
    fn generate_signature(url: &str, secret: &str, expires_at: Option<&DateTime<Utc>>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.update(secret.as_bytes());

        if let Some(expires) = expires_at {
            hasher.update(expires.timestamp().to_string().as_bytes());
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Get the full signed URL with query parameters.
    pub fn to_string(&self) -> String {
        let separator = if self.url.contains('?') { '&' } else { '?' };
        let mut url = format!("{}{}signature={}", self.url, separator, self.signature);

        if let Some(expires) = self.expires_at {
            url.push_str(&format!("&expires={}", expires.timestamp()));
        }

        url
    }

    /// Verify the signature.
    pub fn verify(&self, secret: &str) -> bool {
        let expected_sig = Self::generate_signature(&self.url, secret, self.expires_at.as_ref());

        if self.signature != expected_sig {
            return false;
        }

        // Check expiration
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }

        true
    }

    /// Check if the URL has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Get the signature.
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Get the expiration time.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

/// Builder for creating signed URLs.
pub struct SignedUrlBuilder {
    url: String,
    secret: String,
    expires_in_minutes: Option<i64>,
    expires_at: Option<DateTime<Utc>>,
}

impl SignedUrlBuilder {
    /// Create a new signed URL builder.
    pub fn new(url: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            secret: secret.into(),
            expires_in_minutes: None,
            expires_at: None,
        }
    }

    /// Set expiration time in minutes.
    pub fn expires_in_minutes(mut self, minutes: i64) -> Self {
        self.expires_in_minutes = Some(minutes);
        self
    }

    /// Set expiration time in hours.
    pub fn expires_in_hours(mut self, hours: i64) -> Self {
        self.expires_in_minutes = Some(hours * 60);
        self
    }

    /// Set exact expiration time.
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Build the signed URL.
    pub fn build(self) -> SignedUrl {
        let expires_at = if let Some(expires) = self.expires_at {
            Some(expires)
        } else if let Some(minutes) = self.expires_in_minutes {
            Some(Utc::now() + Duration::minutes(minutes))
        } else {
            None
        };

        SignedUrl::new(self.url, &self.secret, expires_at)
    }
}

/// Parse a signed URL from a string.
pub fn parse_signed_url(url: &str, _secret: &str) -> Option<SignedUrl> {
    // Extract query parameters
    let parts: Vec<&str> = url.splitn(2, '?').collect();
    if parts.len() != 2 {
        return None;
    }

    let base_url = parts[0];
    let query = parts[1];

    let mut params: HashMap<String, String> = HashMap::new();
    for param in query.split('&') {
        let kv: Vec<&str> = param.splitn(2, '=').collect();
        if kv.len() == 2 {
            params.insert(kv[0].to_string(), kv[1].to_string());
        }
    }

    let signature = params.get("signature")?.clone();
    let expires_at = params
        .get("expires")
        .and_then(|s| s.parse::<i64>().ok())
        .map(|timestamp| DateTime::from_timestamp(timestamp, 0).unwrap_or_else(|| Utc::now()));

    // Reconstruct the original URL preserving query params other than signature/expires
    let original_params: Vec<String> = params
        .iter()
        .filter(|(k, _)| k.as_str() != "signature" && k.as_str() != "expires")
        .map(|(k, v)| format!("{}={}", k, v))
        .collect();

    let original_url = if original_params.is_empty() {
        base_url.to_string()
    } else {
        format!("{}?{}", base_url, original_params.join("&"))
    };

    Some(SignedUrl {
        url: original_url,
        signature,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key";

    #[test]
    fn test_signed_url_creation() {
        let signed = SignedUrl::new("/users/123", TEST_SECRET, None);
        assert!(!signed.signature().is_empty());
    }

    #[test]
    fn test_signed_url_to_string() {
        let signed = SignedUrl::new("/users/123", TEST_SECRET, None);
        let url_string = signed.to_string();

        assert!(url_string.starts_with("/users/123?"));
        assert!(url_string.contains("signature="));
    }

    #[test]
    fn test_signed_url_verification() {
        let signed = SignedUrl::new("/users/123", TEST_SECRET, None);
        assert!(signed.verify(TEST_SECRET));
        assert!(!signed.verify("wrong-secret"));
    }

    #[test]
    fn test_signed_url_with_expiration() {
        let expires = Utc::now() + Duration::hours(1);
        let signed = SignedUrl::new("/users/123", TEST_SECRET, Some(expires));

        assert!(!signed.is_expired());
        assert!(signed.verify(TEST_SECRET));
    }

    #[test]
    fn test_signed_url_expired() {
        let expires = Utc::now() - Duration::hours(1);
        let signed = SignedUrl::new("/users/123", TEST_SECRET, Some(expires));

        assert!(signed.is_expired());
        assert!(!signed.verify(TEST_SECRET));
    }

    #[test]
    fn test_signed_url_builder() {
        let signed = SignedUrlBuilder::new("/users/123", TEST_SECRET)
            .expires_in_minutes(60)
            .build();

        assert!(signed.expires_at().is_some());
        assert!(!signed.is_expired());
    }

    #[test]
    fn test_signed_url_builder_hours() {
        let signed = SignedUrlBuilder::new("/users/123", TEST_SECRET)
            .expires_in_hours(2)
            .build();

        assert!(signed.expires_at().is_some());
        assert!(!signed.is_expired());
    }

    #[test]
    fn test_parse_signed_url() {
        let signed = SignedUrl::new("/users/123", TEST_SECRET, None);
        let url_string = signed.to_string();

        let parsed = parse_signed_url(&url_string, TEST_SECRET);
        assert!(parsed.is_some());

        let parsed = parsed.unwrap();
        assert_eq!(parsed.url, "/users/123");
        assert!(parsed.verify(TEST_SECRET));
    }
}
