//! Conversation messages and content blocks.
//!
//! Mirrors the Anthropic Messages API content model: a [`Message`] carries a
//! [`Role`] and a list of [`ContentBlock`]s. Anthropic accepts message content
//! either as a plain string or as an array of typed blocks; we always serialize
//! the array form so tool-calling round-trips are lossless.

use serde::{Deserialize, Serialize};

/// Who authored a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A message from the end user (or a tool result sent back as the user).
    User,
    /// A message from the assistant (the model).
    Assistant,
}

/// A single block of message content.
///
/// The `type` discriminator and `snake_case` variants match the Anthropic wire
/// format exactly (`text`, `tool_use`, `tool_result`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text.
    Text {
        /// The text content.
        text: String,
    },
    /// A request from the model to call a tool.
    ToolUse {
        /// Unique id for this tool invocation (echoed back in the result).
        id: String,
        /// Name of the tool the model wants to call.
        name: String,
        /// Arbitrary JSON arguments for the tool.
        input: serde_json::Value,
    },
    /// The result of a tool call, sent back to the model in a user message.
    ToolResult {
        /// The `id` of the originating [`ContentBlock::ToolUse`].
        tool_use_id: String,
        /// The tool's output, serialized as a string.
        content: String,
        /// Whether the tool execution errored.
        #[serde(default)]
        is_error: bool,
    },
}

impl ContentBlock {
    /// Build a [`ContentBlock::Text`].
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    /// Build a [`ContentBlock::ToolResult`] (non-error).
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        ContentBlock::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    /// Build an error [`ContentBlock::ToolResult`].
    pub fn tool_error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        ContentBlock::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error: true,
        }
    }
}

/// A conversation message: a role plus its content blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Who authored the message.
    pub role: Role,
    /// The message content, as an array of blocks.
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// A user message containing a single text block.
    pub fn user(text: impl Into<String>) -> Self {
        Message {
            role: Role::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// An assistant message containing a single text block.
    pub fn assistant(text: impl Into<String>) -> Self {
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// A message with an explicit role and arbitrary blocks.
    pub fn with_blocks(role: Role, content: Vec<ContentBlock>) -> Self {
        Message { role, content }
    }
}
