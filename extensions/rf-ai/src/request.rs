//! The chat request type and its builder.

use serde::Serialize;

use crate::message::Message;
use crate::tool::{Tool, ToolChoice};

/// The default model used by [`ChatRequest::default_model`] and
/// [`crate::provider::AnthropicProvider`] when no model is specified.
pub const DEFAULT_MODEL: &str = "claude-opus-4-8";

/// Default `max_tokens` for a freshly constructed [`ChatRequest`].
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// A request to the chat/messages endpoint.
///
/// Optional and empty fields are omitted from the serialized JSON so the wire
/// shape matches what Anthropic expects (no `system`, `tools`, or `tool_choice`
/// keys unless they carry content). Note that sampling parameters
/// (`temperature`/`top_p`/`top_k`) are intentionally not modeled: the default
/// model rejects them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChatRequest {
    /// The model id (e.g. `claude-opus-4-8`).
    pub model: String,
    /// Maximum number of tokens to generate.
    pub max_tokens: u32,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// The conversation so far.
    pub messages: Vec<Message>,
    /// Tools the model may call.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    /// How the model should choose among tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

impl ChatRequest {
    /// Start a new request for `model` with default `max_tokens` and no messages.
    pub fn new(model: impl Into<String>) -> Self {
        ChatRequest {
            model: model.into(),
            max_tokens: DEFAULT_MAX_TOKENS,
            system: None,
            messages: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
        }
    }

    /// Start a new request using [`DEFAULT_MODEL`].
    pub fn default_model() -> Self {
        ChatRequest::new(DEFAULT_MODEL)
    }

    /// Set `max_tokens`.
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Append a single message.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Append multiple messages.
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Append a single tool.
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Append multiple tools.
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Set the tool choice.
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }
}
