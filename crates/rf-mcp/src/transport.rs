//! MCP Transport implementations

use crate::errors::{McpError, McpResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Option<serde_json::Value>, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

/// Transport trait for MCP communication
#[async_trait]
pub trait Transport: Send + Sync {
    /// Read a message from the transport
    async fn read(&mut self) -> McpResult<Option<JsonRpcRequest>>;

    /// Write a message to the transport
    async fn write(&mut self, response: JsonRpcResponse) -> McpResult<()>;

    /// Close the transport
    async fn close(&mut self) -> McpResult<()>;
}

/// Standard I/O transport
pub struct StdioTransport {
    stdin: BufReader<tokio::io::Stdin>,
    stdout: tokio::io::Stdout,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(tokio::io::stdin()),
            stdout: tokio::io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn read(&mut self) -> McpResult<Option<JsonRpcRequest>> {
        let mut line = String::new();
        let bytes_read = self.stdin.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let request: JsonRpcRequest =
            serde_json::from_str(&line).map_err(|e| McpError::ProtocolError(e.to_string()))?;

        Ok(Some(request))
    }

    async fn write(&mut self, response: JsonRpcResponse) -> McpResult<()> {
        let json = serde_json::to_string(&response)?;
        self.stdout.write_all(json.as_bytes()).await?;
        self.stdout.write_all(b"\n").await?;
        self.stdout.flush().await?;
        Ok(())
    }

    async fn close(&mut self) -> McpResult<()> {
        Ok(())
    }
}
