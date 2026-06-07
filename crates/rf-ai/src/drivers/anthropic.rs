//! Anthropic Claude API driver.
//!
//! Enabled via the `anthropic` feature flag.

#[cfg(feature = "anthropic")]
use crate::{
    driver::AiDriver,
    embedding::{Embedding, EmbeddingRequest, EmbeddingResponse},
    error::{AiError, AiResult},
    text::{Message, TextRequest, TextResponse},
    tool::{Tool, ToolCall, ToolCallResponse},
};
#[cfg(feature = "anthropic")]
use async_trait::async_trait;
#[cfg(feature = "anthropic")]
use reqwest::Client;

/// Driver for the Anthropic Claude API.
#[cfg(feature = "anthropic")]
pub struct AnthropicDriver {
    api_key: String,
    client: Client,
    base_url: String,
}

#[cfg(feature = "anthropic")]
impl AnthropicDriver {
    /// Create a new driver with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    /// Override the base URL (useful for testing with a mock server).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

#[cfg(feature = "anthropic")]
#[async_trait]
impl AiDriver for AnthropicDriver {
    async fn generate_text(&self, req: TextRequest) -> AiResult<TextResponse> {
        // Build request body following the Anthropic Messages API.
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .filter_map(|m| match m {
                Message::User { content } => {
                    Some(serde_json::json!({ "role": "user", "content": content }))
                }
                Message::Assistant { content } => {
                    Some(serde_json::json!({ "role": "assistant", "content": content }))
                }
                // System messages are passed via the top-level `system` field.
                Message::System { .. } => None,
            })
            .collect();

        let system = req.system.or_else(|| {
            req.messages.iter().find_map(|m| {
                if let Message::System { content } = m {
                    Some(content.clone())
                } else {
                    None
                }
            })
        });

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(1024),
        });

        if let Some(sys) = system {
            body["system"] = serde_json::json!(sys);
        }
        if let Some(temp) = req.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status != 200 {
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError { status, message: text });
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiError::Serialization(e.to_string()))?;

        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(TextResponse {
            content,
            model: json["model"].as_str().unwrap_or(&req.model).to_string(),
            prompt_tokens: json["usage"]["input_tokens"].as_u64().map(|n| n as u32),
            completion_tokens: json["usage"]["output_tokens"].as_u64().map(|n| n as u32),
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }

    async fn embed(&self, _req: EmbeddingRequest) -> AiResult<EmbeddingResponse> {
        // Anthropic does not currently offer an embeddings endpoint.
        Err(AiError::Other(
            "Anthropic does not support embeddings. Use OpenAI or another provider.".to_string(),
        ))
    }

    async fn generate_with_tools(
        &self,
        req: TextRequest,
        tools: Vec<Tool>,
    ) -> AiResult<ToolCallResponse> {
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .filter_map(|m| match m {
                Message::User { content } => {
                    Some(serde_json::json!({ "role": "user", "content": content }))
                }
                Message::Assistant { content } => {
                    Some(serde_json::json!({ "role": "assistant", "content": content }))
                }
                Message::System { .. } => None,
            })
            .collect();

        let tools_json: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "tools": tools_json,
            "max_tokens": req.max_tokens.unwrap_or(1024),
        });

        if let Some(temp) = req.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status != 200 {
            let text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError { status, message: text });
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiError::Serialization(e.to_string()))?;

        let mut text_content: Option<String> = None;
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        if let Some(blocks) = json["content"].as_array() {
            for block in blocks {
                match block["type"].as_str() {
                    Some("text") => {
                        text_content = block["text"].as_str().map(|s| s.to_string());
                    }
                    Some("tool_use") => {
                        tool_calls.push(ToolCall {
                            id: block["id"].as_str().unwrap_or("").to_string(),
                            name: block["name"].as_str().unwrap_or("").to_string(),
                            arguments: block["input"].clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(ToolCallResponse {
            content: text_content,
            tool_calls,
            model: json["model"].as_str().unwrap_or(&req.model).to_string(),
            finish_reason: json["stop_reason"].as_str().map(|s| s.to_string()),
        })
    }
}

// Keep the module compilable even without the feature flag.
#[cfg(not(feature = "anthropic"))]
pub struct AnthropicDriver;
