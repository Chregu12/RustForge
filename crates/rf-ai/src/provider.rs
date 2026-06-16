//! Provider traits and the live Anthropic provider.

use async_trait::async_trait;

use crate::error::{AiError, AiResult};
use crate::request::ChatRequest;
use crate::response::ChatResponse;

/// Something that can answer a [`ChatRequest`].
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Send a chat request and await the response.
    async fn chat(&self, request: &ChatRequest) -> AiResult<ChatResponse>;
}

/// Something that can produce embeddings for text.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed each input string into a vector of floats.
    async fn embed(&self, texts: &[String]) -> AiResult<Vec<Vec<f32>>>;
}

/// A [`ChatProvider`] that talks to the real Anthropic Messages API over HTTP.
///
/// Rust has no official Anthropic SDK, so this issues raw `reqwest` calls to
/// `POST {base_url}/v1/messages` with the required headers.
pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
    version: String,
    http: reqwest::Client,
}

impl AnthropicProvider {
    /// Construct a provider with the default base URL and API version.
    pub fn new(api_key: impl Into<String>) -> Self {
        AnthropicProvider {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".to_string(),
            version: "2023-06-01".to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Override the base URL (useful for proxies or testing).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[async_trait]
impl ChatProvider for AnthropicProvider {
    async fn chat(&self, request: &ChatRequest) -> AiResult<ChatResponse> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AiError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let parsed = response.json::<ChatResponse>().await?;
        Ok(parsed)
    }
}
