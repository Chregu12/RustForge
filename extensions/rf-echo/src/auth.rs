//! Authentication for private and presence channels

use crate::{EchoError, EchoResult};
use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashMap;

/// Authentication provider trait
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Authenticate a channel subscription
    async fn authenticate(&self, channel: &str, socket_id: &str) -> EchoResult<String>;

    /// Authenticate with user data (for presence channels)
    async fn authenticate_presence(
        &self,
        channel: &str,
        socket_id: &str,
        user_info: &PresenceUserInfo,
    ) -> EchoResult<PresenceAuth>;
}

/// User info for presence channels
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresenceUserInfo {
    pub user_id: String,
    pub user_info: serde_json::Value,
}

/// Authentication response for presence channels
#[derive(Debug, Clone)]
pub struct PresenceAuth {
    pub auth: String,
    pub channel_data: String,
}

/// Default HTTP-based authentication provider
pub struct DefaultAuthProvider {
    auth_endpoint: Option<String>,
    headers: HashMap<String, String>,
    csrf_token: Option<String>,
}

impl DefaultAuthProvider {
    pub fn new(
        auth_endpoint: Option<String>,
        headers: HashMap<String, String>,
        csrf_token: Option<String>,
    ) -> Self {
        Self {
            auth_endpoint,
            headers,
            csrf_token,
        }
    }
}

#[async_trait]
impl AuthProvider for DefaultAuthProvider {
    async fn authenticate(&self, channel: &str, socket_id: &str) -> EchoResult<String> {
        let endpoint = self
            .auth_endpoint
            .as_ref()
            .ok_or_else(|| EchoError::AuthError("No auth endpoint configured".to_string()))?;

        let client = reqwest::Client::new();
        let mut request = client.post(endpoint).form(&[
            ("socket_id", socket_id),
            ("channel_name", channel),
        ]);

        // Add headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Add CSRF token if available
        if let Some(ref token) = self.csrf_token {
            request = request.header("X-CSRF-TOKEN", token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| EchoError::AuthError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(EchoError::AuthError(format!(
                "Authentication failed: {}",
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct AuthResponse {
            auth: String,
        }

        let auth_response: AuthResponse = response
            .json()
            .await
            .map_err(|e| EchoError::AuthError(e.to_string()))?;

        Ok(auth_response.auth)
    }

    async fn authenticate_presence(
        &self,
        channel: &str,
        socket_id: &str,
        user_info: &PresenceUserInfo,
    ) -> EchoResult<PresenceAuth> {
        let endpoint = self
            .auth_endpoint
            .as_ref()
            .ok_or_else(|| EchoError::AuthError("No auth endpoint configured".to_string()))?;

        let channel_data = serde_json::json!({
            "user_id": user_info.user_id,
            "user_info": user_info.user_info,
        });

        let client = reqwest::Client::new();
        let mut request = client.post(endpoint).form(&[
            ("socket_id", socket_id),
            ("channel_name", channel),
            ("channel_data", &channel_data.to_string()),
        ]);

        // Add headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Add CSRF token if available
        if let Some(ref token) = self.csrf_token {
            request = request.header("X-CSRF-TOKEN", token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| EchoError::AuthError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(EchoError::AuthError(format!(
                "Authentication failed: {}",
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct AuthResponse {
            auth: String,
            channel_data: String,
        }

        let auth_response: AuthResponse = response
            .json()
            .await
            .map_err(|e| EchoError::AuthError(e.to_string()))?;

        Ok(PresenceAuth {
            auth: auth_response.auth,
            channel_data: auth_response.channel_data,
        })
    }
}

/// Local authentication provider (for testing or self-hosted)
pub struct LocalAuthProvider {
    app_key: String,
    app_secret: String,
}

impl LocalAuthProvider {
    pub fn new(app_key: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Self {
            app_key: app_key.into(),
            app_secret: app_secret.into(),
        }
    }

    /// Generate auth signature for private channel
    pub fn sign(&self, socket_id: &str, channel: &str) -> String {
        let string_to_sign = format!("{}:{}", socket_id, channel);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac =
            HmacSha256::new_from_slice(self.app_secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        format!("{}:{}", self.app_key, signature)
    }

    /// Generate auth signature for presence channel
    pub fn sign_presence(&self, socket_id: &str, channel: &str, channel_data: &str) -> String {
        let string_to_sign = format!("{}:{}:{}", socket_id, channel, channel_data);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac =
            HmacSha256::new_from_slice(self.app_secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        format!("{}:{}", self.app_key, signature)
    }
}

#[async_trait]
impl AuthProvider for LocalAuthProvider {
    async fn authenticate(&self, channel: &str, socket_id: &str) -> EchoResult<String> {
        Ok(self.sign(socket_id, channel))
    }

    async fn authenticate_presence(
        &self,
        channel: &str,
        socket_id: &str,
        user_info: &PresenceUserInfo,
    ) -> EchoResult<PresenceAuth> {
        let channel_data = serde_json::json!({
            "user_id": user_info.user_id,
            "user_info": user_info.user_info,
        })
        .to_string();

        let auth = self.sign_presence(socket_id, channel, &channel_data);

        Ok(PresenceAuth { auth, channel_data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_auth_provider_sign() {
        let provider = LocalAuthProvider::new("app-key", "secret");
        let auth = provider.sign("socket-id", "private-channel");
        assert!(auth.starts_with("app-key:"));
    }

    #[test]
    fn test_local_auth_provider_sign_presence() {
        let provider = LocalAuthProvider::new("app-key", "secret");
        let auth = provider.sign_presence("socket-id", "presence-channel", r#"{"user_id":"1"}"#);
        assert!(auth.starts_with("app-key:"));
    }
}
