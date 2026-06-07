//! OpenAI API driver.
//!
//! Enabled via the `openai` feature flag.

#[cfg(feature = "openai")]
use crate::{
    driver::AiDriver,
    embedding::{Embedding, EmbeddingRequest, EmbeddingResponse},
    error::{AiError, AiResult},
    text::{Message, TextRequest, TextResponse},
    tool::{Tool, ToolCall, ToolCallResponse},
};
#[cfg(feature = "openai")]
use async_trait::async_trait;
#[cfg(feature = "openai")]
use reqwest::Client;

/// Driver for the OpenAI Chat Completions API.
#[cfg(feature = "openai")]
pub struct OpenAiDriver {
    api_key: String,
    client: Client,
    base_url: String,
}

#[cfg(feature = "openai")]
impl OpenAiDriver {
    /// Create a new driver with the given API key.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            base_url: "https://api.openai.com".to_string(),
        }
    }

    /// Override the base URL (useful for testing with a mock server or
    /// Azure OpenAI endpoints).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    fn messages_to_json(messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| match m {
                Message::System { content } => {
                    serde_json::json!({ "role": "system", "content": content })
                }
                Message::User { content } => {
                    serde_json::json!({ "role": "user", "content": content })
                }
                Message::Assistant { content } => {
                    serde_json::json!({ "role": "assistant", "content": content })
                }
            })
            .collect()
    }
}

#[cfg(feature = "openai")]
#[async_trait]
impl AiDriver for OpenAiDriver {
    async fn generate_text(&self, req: TextRequest) -> AiResult<TextResponse> {
        let mut messages = Self::messages_to_json(&req.messages);

        // Prepend system message if provided via `TextRequest::system`.
        if let Some(sys) = &req.system {
            messages.insert(0, serde_json::json!({ "role": "system", "content": sys }));
        }

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
        });
        if let Some(temp) = req.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max) = req.max_tokens {
            body["max_tokens"] = serde_json::json!(max);
        }

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
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

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(TextResponse {
            content,
            model: json["model"].as_str().unwrap_or(&req.model).to_string(),
            prompt_tokens: json["usage"]["prompt_tokens"].as_u64().map(|n| n as u32),
            completion_tokens: json["usage"]["completion_tokens"]
                .as_u64()
                .map(|n| n as u32),
            finish_reason: json["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    async fn embed(&self, req: EmbeddingRequest) -> AiResult<EmbeddingResponse> {
        let body = serde_json::json!({
            "model": req.model,
            "input": req.input,
        });

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .bearer_auth(&self.api_key)
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

        let embeddings = json["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let vector = item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();
                Embedding { vector, index: item["index"].as_u64().unwrap_or(i as u64) as usize }
            })
            .collect();

        Ok(EmbeddingResponse {
            embeddings,
            model: json["model"].as_str().unwrap_or(&req.model).to_string(),
            total_tokens: json["usage"]["total_tokens"].as_u64().map(|n| n as u32),
        })
    }

    async fn generate_with_tools(
        &self,
        req: TextRequest,
        tools: Vec<Tool>,
    ) -> AiResult<ToolCallResponse> {
        let mut messages = Self::messages_to_json(&req.messages);
        if let Some(sys) = &req.system {
            messages.insert(0, serde_json::json!({ "role": "system", "content": sys }));
        }

        let functions: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "tools": functions,
        });
        if let Some(temp) = req.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(max) = req.max_tokens {
            body["max_tokens"] = serde_json::json!(max);
        }

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
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

        let choice = &json["choices"][0];
        let message = &choice["message"];

        let text_content = message["content"].as_str().map(|s| s.to_string());

        let tool_calls = message["tool_calls"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|tc| {
                let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                let arguments =
                    serde_json::from_str(args_str).unwrap_or(serde_json::Value::Object(
                        serde_json::Map::new(),
                    ));
                ToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments,
                }
            })
            .collect();

        Ok(ToolCallResponse {
            content: text_content,
            tool_calls,
            model: json["model"].as_str().unwrap_or(&req.model).to_string(),
            finish_reason: choice["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }
}

// Keep the module compilable even without the feature flag.
#[cfg(not(feature = "openai"))]
pub struct OpenAiDriver;
