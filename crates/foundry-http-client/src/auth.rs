//! Authentication helpers

use base64::Engine;
use reqwest::header::{HeaderValue, AUTHORIZATION};

#[derive(Debug, Clone)]
pub enum AuthType {
    Bearer(String),
    Basic { username: String, password: String },
    Custom { header: String, value: String },
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct Auth {
    auth_type: AuthType,
}

impl Auth {
    pub fn new(auth_type: AuthType) -> Self {
        Self { auth_type }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Self::new(AuthType::Bearer(token.into()))
    }

    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::new(AuthType::Basic {
            username: username.into(),
            password: password.into(),
        })
    }

    pub fn custom(header: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(AuthType::Custom {
            header: header.into(),
            value: value.into(),
        })
    }

    pub fn apply(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        match &self.auth_type {
            AuthType::Bearer(token) => Ok(request.bearer_auth(token)),
            AuthType::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded =
                    base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
                let header_value = HeaderValue::from_str(&format!("Basic {}", encoded))?;
                Ok(request.header(AUTHORIZATION, header_value))
            }
            AuthType::Custom { header, value } => {
                let header_value = HeaderValue::from_str(value)?;
                Ok(request.header(header, header_value))
            }
        }
    }
}
