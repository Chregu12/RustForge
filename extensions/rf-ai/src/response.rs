//! The chat response type returned by providers.

use serde::Deserialize;

use crate::message::{ContentBlock, Role};

/// Token usage reported by the provider.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt.
    #[serde(default)]
    pub input_tokens: u32,
    /// Tokens generated in the response.
    #[serde(default)]
    pub output_tokens: u32,
}

/// A response from the chat/messages endpoint.
///
/// Mirrors the Anthropic Messages API response. Missing or absent fields are
/// tolerated via serde defaults where reasonable so mock and partial payloads
/// deserialize cleanly.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ChatResponse {
    /// Provider-assigned message id.
    #[serde(default)]
    pub id: String,
    /// The model that produced the response.
    #[serde(default)]
    pub model: String,
    /// Always [`Role::Assistant`] for a response.
    #[serde(default = "assistant_role")]
    pub role: Role,
    /// Why the model stopped (`end_turn`, `tool_use`, `max_tokens`, ...).
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// The response content blocks.
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    /// Token usage.
    #[serde(default)]
    pub usage: Usage,
}

fn assistant_role() -> Role {
    Role::Assistant
}

impl ChatResponse {
    /// Concatenate the text of all [`ContentBlock::Text`] blocks.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for block in &self.content {
            if let ContentBlock::Text { text } = block {
                out.push_str(text);
            }
        }
        out
    }

    /// All [`ContentBlock::ToolUse`] blocks in the response.
    pub fn tool_uses(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
            .collect()
    }

    /// Whether the model stopped because it wants to call tools.
    pub fn stopped_for_tools(&self) -> bool {
        self.stop_reason.as_deref() == Some("tool_use")
    }
}
