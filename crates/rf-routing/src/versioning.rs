//! API Versioning Support
//!
//! Supports both header-based and URL-based versioning

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use regex::Regex;
use std::sync::OnceLock;
use thiserror::Error;

/// API version extractor
///
/// Extracts version from:
/// 1. Accept header: `Accept: application/vnd.api.v1+json`
/// 2. Custom header: `API-Version: 1`
/// 3. URL path: `/v1/users`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion(pub u32);

impl ApiVersion {
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    pub fn version(&self) -> u32 {
        self.0
    }

    /// Check if this version matches
    pub fn is(&self, version: u32) -> bool {
        self.0 == version
    }

    /// Check if this version is at least the specified version
    pub fn at_least(&self, version: u32) -> bool {
        self.0 >= version
    }
}

#[derive(Error, Debug)]
pub enum VersionError {
    #[error("API version not specified")]
    MissingVersion,

    #[error("Invalid API version format")]
    InvalidFormat,

    #[error("Unsupported API version: {0}")]
    UnsupportedVersion(u32),

    #[error("API version {0} is deprecated")]
    DeprecatedVersion(u32),
}

impl IntoResponse for VersionError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            VersionError::MissingVersion => (StatusCode::BAD_REQUEST, self.to_string()),
            VersionError::InvalidFormat => (StatusCode::BAD_REQUEST, self.to_string()),
            VersionError::UnsupportedVersion(_) => (StatusCode::NOT_ACCEPTABLE, self.to_string()),
            VersionError::DeprecatedVersion(_) => (StatusCode::GONE, self.to_string()),
        };

        (status, message).into_response()
    }
}

/// Version configuration
#[derive(Clone)]
pub struct VersionConfig {
    pub default_version: u32,
    pub supported_versions: Vec<u32>,
    pub deprecated_versions: Vec<u32>,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            default_version: 1,
            supported_versions: vec![1],
            deprecated_versions: vec![],
        }
    }
}

static VERSION_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_version_regex() -> &'static Regex {
    VERSION_REGEX.get_or_init(|| Regex::new(r"application/vnd\.api\.v(\d+)\+json").unwrap())
}

/// Extract version from Accept header
///
/// Format: `Accept: application/vnd.api.v1+json`
pub fn extract_from_accept(accept: &str) -> Option<u32> {
    let regex = get_version_regex();
    regex
        .captures(accept)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Extract version from custom API-Version header
///
/// Format: `API-Version: 1`
pub fn extract_from_header(version: &str) -> Option<u32> {
    version.parse::<u32>().ok()
}

#[async_trait::async_trait]
impl<S> FromRequestParts<S> for ApiVersion
where
    S: Send + Sync,
{
    type Rejection = VersionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try Accept header first
        if let Some(accept) = parts.headers.get(header::ACCEPT) {
            if let Ok(accept_str) = accept.to_str() {
                if let Some(version) = extract_from_accept(accept_str) {
                    return Ok(ApiVersion(version));
                }
            }
        }

        // Try custom API-Version header
        if let Some(version_header) = parts.headers.get("API-Version") {
            if let Ok(version_str) = version_header.to_str() {
                if let Some(version) = extract_from_header(version_str) {
                    return Ok(ApiVersion(version));
                }
            }
        }

        // Try extracting from path (e.g., /v1/users)
        let path = parts.uri.path();
        if let Some(version) = extract_from_path(path) {
            return Ok(ApiVersion(version));
        }

        Err(VersionError::MissingVersion)
    }
}

/// Extract version from URL path
///
/// Supports patterns like:
/// - `/v1/users`
/// - `/api/v2/posts`
pub fn extract_from_path(path: &str) -> Option<u32> {
    static PATH_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = PATH_REGEX.get_or_init(|| Regex::new(r"/v(\d+)/").unwrap());

    regex
        .captures(path)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Version negotiation strategy
pub trait VersionNegotiator {
    fn negotiate(&self, requested: Option<u32>) -> Result<u32, VersionError>;
    fn is_supported(&self, version: u32) -> bool;
    fn is_deprecated(&self, version: u32) -> bool;
}

/// Default version negotiator
pub struct DefaultNegotiator {
    config: VersionConfig,
}

impl DefaultNegotiator {
    pub fn new(config: VersionConfig) -> Self {
        Self { config }
    }
}

impl VersionNegotiator for DefaultNegotiator {
    fn negotiate(&self, requested: Option<u32>) -> Result<u32, VersionError> {
        match requested {
            Some(v) => {
                if self.is_deprecated(v) {
                    Err(VersionError::DeprecatedVersion(v))
                } else if self.is_supported(v) {
                    Ok(v)
                } else {
                    Err(VersionError::UnsupportedVersion(v))
                }
            }
            None => Ok(self.config.default_version),
        }
    }

    fn is_supported(&self, version: u32) -> bool {
        self.config.supported_versions.contains(&version)
    }

    fn is_deprecated(&self, version: u32) -> bool {
        self.config.deprecated_versions.contains(&version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_accept() {
        let accept = "application/vnd.api.v1+json";
        assert_eq!(extract_from_accept(accept), Some(1));

        let accept = "application/vnd.api.v2+json";
        assert_eq!(extract_from_accept(accept), Some(2));

        let accept = "application/json";
        assert_eq!(extract_from_accept(accept), None);
    }

    #[test]
    fn test_extract_from_header() {
        assert_eq!(extract_from_header("1"), Some(1));
        assert_eq!(extract_from_header("42"), Some(42));
        assert_eq!(extract_from_header("invalid"), None);
    }

    #[test]
    fn test_extract_from_path() {
        assert_eq!(extract_from_path("/v1/users"), Some(1));
        assert_eq!(extract_from_path("/api/v2/posts"), Some(2));
        assert_eq!(extract_from_path("/users"), None);
    }

    #[test]
    fn test_version_negotiation() {
        let config = VersionConfig {
            default_version: 1,
            supported_versions: vec![1, 2, 3],
            deprecated_versions: vec![0],
        };
        let negotiator = DefaultNegotiator::new(config);

        assert_eq!(negotiator.negotiate(Some(1)).unwrap(), 1);
        assert_eq!(negotiator.negotiate(Some(2)).unwrap(), 2);
        assert_eq!(negotiator.negotiate(None).unwrap(), 1);
        assert!(negotiator.negotiate(Some(0)).is_err());
        assert!(negotiator.negotiate(Some(99)).is_err());
    }
}
