//! MCP Server implementation

use crate::config::get_config;
use crate::errors::{McpError, McpResult};
use crate::prompt::PromptRegistry;
use crate::resource::ResourceRegistry;
use crate::tool::ToolRegistry;
use crate::transport::{JsonRpcResponse, StdioTransport, Transport};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};

/// MCP Server
pub struct McpServer {
    tools: Arc<ToolRegistry>,
    resources: Arc<ResourceRegistry>,
    prompts: Arc<PromptRegistry>,
}

/// Server info for initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self {
            tools: ToolRegistry::global(),
            resources: ResourceRegistry::global(),
            prompts: PromptRegistry::global(),
        }
    }

    /// Run the MCP server
    pub async fn run(&self) -> McpResult<()> {
        info!("Starting MCP server");

        let mut transport = StdioTransport::new();

        loop {
            match transport.read().await {
                Ok(Some(request)) => {
                    let response = self.handle_request(&request.method, request.params).await;
                    let json_response = match response {
                        Ok(result) => JsonRpcResponse::success(request.id, result),
                        Err(e) => JsonRpcResponse::error(request.id, -32000, &e.to_string()),
                    };
                    if let Err(e) = transport.write(json_response).await {
                        error!("Failed to write response: {}", e);
                    }
                }
                Ok(None) => {
                    info!("Transport closed");
                    break;
                }
                Err(e) => {
                    error!("Failed to read request: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Handle a JSON-RPC request
    async fn handle_request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        match method {
            "initialize" => self.handle_initialize(params).await,
            "initialized" => Ok(json!({})),
            "tools/list" => self.handle_tools_list().await,
            "tools/call" => self.handle_tool_call(params).await,
            "resources/list" => self.handle_resources_list().await,
            "resources/read" => self.handle_resource_read(params).await,
            "prompts/list" => self.handle_prompts_list().await,
            "prompts/get" => self.handle_prompt_get(params).await,
            "ping" => Ok(json!({})),
            _ => Err(McpError::ProtocolError(format!(
                "Unknown method: {}",
                method
            ))),
        }
    }

    /// Handle initialize request
    async fn handle_initialize(
        &self,
        _params: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let config = get_config();

        let capabilities = ServerCapabilities {
            tools: if !self.tools.list().is_empty() {
                Some(ToolsCapability { list_changed: true })
            } else {
                None
            },
            resources: if !self.resources.list().is_empty() {
                Some(ResourcesCapability {
                    subscribe: false,
                    list_changed: true,
                })
            } else {
                None
            },
            prompts: if !self.prompts.list().is_empty() {
                Some(PromptsCapability { list_changed: true })
            } else {
                None
            },
        };

        Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": capabilities,
            "serverInfo": {
                "name": config.name,
                "version": config.version
            }
        }))
    }

    /// Handle tools/list request
    async fn handle_tools_list(&self) -> McpResult<serde_json::Value> {
        let tools: Vec<serde_json::Value> = self
            .tools
            .list()
            .into_iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema
                })
            })
            .collect();

        Ok(json!({ "tools": tools }))
    }

    /// Handle tools/call request
    async fn handle_tool_call(
        &self,
        params: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let params = params.ok_or_else(|| McpError::InvalidInput("Missing params".to_string()))?;

        let name = params["name"]
            .as_str()
            .ok_or_else(|| McpError::InvalidInput("Missing tool name".to_string()))?;

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        let result = self.tools.execute(name, arguments).await?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&result)?
            }]
        }))
    }

    /// Handle resources/list request
    async fn handle_resources_list(&self) -> McpResult<serde_json::Value> {
        let resources: Vec<serde_json::Value> = self
            .resources
            .list()
            .into_iter()
            .map(|resource| {
                json!({
                    "uri": resource.uri_template,
                    "name": resource.name,
                    "description": resource.description,
                    "mimeType": resource.mime_type
                })
            })
            .collect();

        Ok(json!({ "resources": resources }))
    }

    /// Handle resources/read request
    async fn handle_resource_read(
        &self,
        params: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let params = params.ok_or_else(|| McpError::InvalidInput("Missing params".to_string()))?;

        let uri = params["uri"]
            .as_str()
            .ok_or_else(|| McpError::InvalidInput("Missing resource URI".to_string()))?;

        let content = self.resources.fetch(uri).await?;

        Ok(json!({
            "contents": [content]
        }))
    }

    /// Handle prompts/list request
    async fn handle_prompts_list(&self) -> McpResult<serde_json::Value> {
        let prompts: Vec<serde_json::Value> = self
            .prompts
            .list()
            .into_iter()
            .map(|prompt| {
                json!({
                    "name": prompt.name,
                    "description": prompt.description,
                    "arguments": prompt.arguments
                })
            })
            .collect();

        Ok(json!({ "prompts": prompts }))
    }

    /// Handle prompts/get request
    async fn handle_prompt_get(
        &self,
        params: Option<serde_json::Value>,
    ) -> McpResult<serde_json::Value> {
        let params = params.ok_or_else(|| McpError::InvalidInput("Missing params".to_string()))?;

        let name = params["name"]
            .as_str()
            .ok_or_else(|| McpError::InvalidInput("Missing prompt name".to_string()))?;

        let arguments: std::collections::HashMap<String, String> = params
            .get("arguments")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let messages = self.prompts.render(name, &arguments)?;

        Ok(json!({
            "messages": messages
        }))
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
