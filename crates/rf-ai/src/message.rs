//! Conversation messages and content blocks.
//!
//! Mirrors the Anthropic Messages API content model: a [`Message`] carries a
//! [`Role`] and a list of [`ContentBlock`]s. Anthropic accepts message content
//! either as a plain string or as an array of typed blocks; we always serialize
//! the array form so tool-calling round-trips are lossless.

use base64::Engine as _;
use serde::{Deserialize, Serialize};

/// Base64-encode `bytes` using the standard alphabet (with padding).
///
/// Used by the [`ContentBlock::image`] / [`ContentBlock::document`] byte helpers
/// to build the base64 payload Anthropic's Messages API expects.
fn base64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Who authored a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A message from the end user (or a tool result sent back as the user).
    User,
    /// A message from the assistant (the model).
    Assistant,
}

/// The source payload for an [`ContentBlock::Image`] or [`ContentBlock::Document`].
///
/// The `type` discriminator and field names match the Anthropic wire format
/// exactly: `{"type":"base64","media_type":"image/png","data":"..."}` for binary
/// data (images, PDFs) and `{"type":"text","media_type":"text/plain","data":"..."}`
/// for an inline plain-text document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// Base64-encoded binary data, e.g. `image/png` or `application/pdf`.
    Base64 {
        /// The MIME type of the data (e.g. `image/png`, `application/pdf`).
        media_type: String,
        /// The base64-encoded bytes.
        data: String,
    },
    /// An inline plain-text document (`media_type` is typically `text/plain`).
    Text {
        /// The MIME type of the text (e.g. `text/plain`).
        media_type: String,
        /// The raw, unencoded text content.
        data: String,
    },
}

/// A single block of message content.
///
/// The `type` discriminator and `snake_case` variants match the Anthropic wire
/// format exactly (`text`, `image`, `document`, `tool_use`, `tool_result`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text.
    Text {
        /// The text content.
        text: String,
    },
    /// An image attachment (`{"type":"image","source":{...}}`).
    Image {
        /// Where the image bytes come from.
        source: Source,
    },
    /// A document attachment — a base64 PDF or an inline plain-text document
    /// (`{"type":"document","source":{...}}`).
    Document {
        /// Where the document comes from.
        source: Source,
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

    /// Build a [`ContentBlock::Image`] from an already-base64-encoded string.
    ///
    /// Use this when you already hold a base64 payload; for raw bytes prefer
    /// [`ContentBlock::image`], which encodes for you.
    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        ContentBlock::Image {
            source: Source::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }

    /// Build a [`ContentBlock::Image`] from raw bytes, base64-encoding them.
    ///
    /// `media_type` is the image MIME type, e.g. `image/png` or `image/jpeg`.
    pub fn image(media_type: impl Into<String>, bytes: impl AsRef<[u8]>) -> Self {
        ContentBlock::image_base64(media_type, base64_encode(bytes.as_ref()))
    }

    /// Build a base64 [`ContentBlock::Document`] from an already-base64 string.
    pub fn document_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        ContentBlock::Document {
            source: Source::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }

    /// Build a base64 [`ContentBlock::Document`] (e.g. a PDF) from raw bytes.
    ///
    /// `media_type` is the document MIME type, typically `application/pdf`.
    pub fn document(media_type: impl Into<String>, bytes: impl AsRef<[u8]>) -> Self {
        ContentBlock::document_base64(media_type, base64_encode(bytes.as_ref()))
    }

    /// Build an inline plain-text [`ContentBlock::Document`] (`text/plain`).
    pub fn text_document(text: impl Into<String>) -> Self {
        ContentBlock::Document {
            source: Source::Text {
                media_type: "text/plain".to_string(),
                data: text.into(),
            },
        }
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
