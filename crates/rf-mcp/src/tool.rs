//! MCP Tool definitions

use crate::errors::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};

type ToolHandler = Arc<
    dyn Fn(
            serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = McpResult<serde_json::Value>> + Send>>
        + Send
        + Sync,
>;

static TOOL_REGISTRY: OnceLock<Arc<ToolRegistry>> = OnceLock::new();

/// Tool input schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// Input properties
    #[serde(rename = "type")]
    pub type_: String,
    pub properties: HashMap<String, PropertySchema>,
    #[serde(default)]
    pub required: Vec<String>,
}

/// Property schema for tool inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// Tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolContent>>,
    #[serde(default)]
    pub is_error: bool,
}

/// Tool content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String, text: Option<String> },
}

impl ToolResult {
    /// Create a text result
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: Some(vec![ToolContent::Text { text: text.into() }]),
            is_error: false,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: Some(vec![ToolContent::Text {
                text: message.into(),
            }]),
            is_error: true,
        }
    }

    /// Create an image result
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            content: Some(vec![ToolContent::Image {
                data: data.into(),
                mime_type: mime_type.into(),
            }]),
            is_error: false,
        }
    }
}

/// A registered tool
#[derive(Clone)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema
    pub input_schema: ToolInput,
    /// Handler function
    #[allow(dead_code)]
    handler: ToolHandler,
}

impl Tool {
    /// Execute the tool
    pub async fn execute(&self, input: serde_json::Value) -> McpResult<serde_json::Value> {
        (self.handler)(input).await
    }
}

/// Tool builder
pub struct ToolBuilder {
    name: String,
    description: Option<String>,
    properties: HashMap<String, PropertySchema>,
    required: Vec<String>,
    handler: Option<ToolHandler>,
}

impl ToolBuilder {
    /// Create a new tool builder
    pub fn new<F>(name: &str, handler: F) -> Self
    where
        F: Fn(
                serde_json::Value,
            ) -> Pin<Box<dyn Future<Output = McpResult<serde_json::Value>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        Self {
            name: name.to_string(),
            description: None,
            properties: HashMap::new(),
            required: Vec::new(),
            handler: Some(Arc::new(handler)),
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Add a string parameter
    pub fn string_param(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.insert(
            name.to_string(),
            PropertySchema {
                type_: "string".to_string(),
                description: Some(description.to_string()),
                enum_values: None,
                default: None,
            },
        );
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add a number parameter
    pub fn number_param(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.insert(
            name.to_string(),
            PropertySchema {
                type_: "number".to_string(),
                description: Some(description.to_string()),
                enum_values: None,
                default: None,
            },
        );
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add a boolean parameter
    pub fn boolean_param(mut self, name: &str, description: &str, required: bool) -> Self {
        self.properties.insert(
            name.to_string(),
            PropertySchema {
                type_: "boolean".to_string(),
                description: Some(description.to_string()),
                enum_values: None,
                default: None,
            },
        );
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Add an enum parameter
    pub fn enum_param(
        mut self,
        name: &str,
        description: &str,
        values: Vec<&str>,
        required: bool,
    ) -> Self {
        self.properties.insert(
            name.to_string(),
            PropertySchema {
                type_: "string".to_string(),
                description: Some(description.to_string()),
                enum_values: Some(values.into_iter().map(|s| s.to_string()).collect()),
                default: None,
            },
        );
        if required {
            self.required.push(name.to_string());
        }
        self
    }

    /// Register the tool
    pub fn register(self) {
        let tool = Tool {
            name: self.name.clone(),
            description: self.description,
            input_schema: ToolInput {
                type_: "object".to_string(),
                properties: self.properties,
                required: self.required,
            },
            handler: self.handler.unwrap(),
        };

        ToolRegistry::global().register(tool);
    }
}

/// Tool registry
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global tool registry
    pub fn global() -> Arc<Self> {
        TOOL_REGISTRY
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    /// Register a tool
    pub fn register(&self, tool: Tool) {
        let mut tools = self.tools.write().unwrap();
        tools.insert(tool.name.clone(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Tool> {
        let tools = self.tools.read().unwrap();
        tools.get(name).cloned()
    }

    /// List all tools
    pub fn list(&self) -> Vec<Tool> {
        let tools = self.tools.read().unwrap();
        tools.values().cloned().collect()
    }

    /// Execute a tool
    pub async fn execute(&self, name: &str, input: serde_json::Value) -> McpResult<serde_json::Value> {
        let tool = self.get(name).ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;
        tool.execute(input).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
