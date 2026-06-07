use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool (function) that can be called by the model during generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Unique name of the tool (must match the name used in the model response).
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema that describes the tool's input parameters.
    pub parameters: Value,
}

impl Tool {
    /// Create a new tool definition.
    ///
    /// `parameters` must be a valid JSON Schema object describing the
    /// expected input, e.g.:
    ///
    /// ```json
    /// {
    ///   "type": "object",
    ///   "properties": {
    ///     "location": { "type": "string", "description": "City name" }
    ///   },
    ///   "required": ["location"]
    /// }
    /// ```
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self { name: name.into(), description: description.into(), parameters }
    }

    /// Build a minimal tool with an empty parameter schema.
    pub fn no_params(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(name, description, serde_json::json!({ "type": "object", "properties": {} }))
    }
}

/// A single tool call produced by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The identifier assigned to this call by the provider.
    pub id: String,
    /// The name of the tool being called.
    pub name: String,
    /// The arguments passed to the tool, as a JSON value.
    pub arguments: Value,
}

/// Response when the model requests tool calls instead of (or in addition to)
/// producing plain text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// Any plain text the model produced before requesting the tool calls.
    pub content: Option<String>,
    /// The tool calls requested by the model.
    pub tool_calls: Vec<ToolCall>,
    /// The model that produced this response.
    pub model: String,
    /// The reason the model stopped generating.
    pub finish_reason: Option<String>,
}
