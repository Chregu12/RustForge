//! Tool definitions and tool-choice configuration.

use serde::{Serialize, Serializer};

/// A tool the model may call.
///
/// `input_schema` is a JSON Schema object describing the tool's parameters,
/// matching Anthropic's tool definition format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tool {
    /// The tool name (must be unique within a request).
    pub name: String,
    /// A description telling the model when and how to use the tool.
    pub description: String,
    /// JSON Schema for the tool's input.
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Construct a new [`Tool`].
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Tool {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Controls whether and which tool the model must use.
///
/// Serializes to Anthropic's `tool_choice` shapes:
/// `{"type":"auto"}`, `{"type":"any"}`, `{"type":"tool","name":"..."}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolChoice {
    /// The model decides whether to use a tool (Anthropic default).
    Auto,
    /// The model must use at least one tool.
    Any,
    /// The model must use the named tool.
    Tool(String),
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        match self {
            ToolChoice::Auto => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "auto")?;
                map.end()
            }
            ToolChoice::Any => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "any")?;
                map.end()
            }
            ToolChoice::Tool(name) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "tool")?;
                map.serialize_entry("name", name)?;
                map.end()
            }
        }
    }
}
