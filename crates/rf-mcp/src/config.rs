//! MCP configuration

use std::sync::OnceLock;

static CONFIG: OnceLock<McpConfig> = OnceLock::new();

/// MCP configuration
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
    /// Server description
    pub description: Option<String>,
    /// Transport type
    pub transport: TransportType,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

/// Transport type for MCP communication
#[derive(Debug, Clone)]
pub enum TransportType {
    /// Standard I/O (stdin/stdout)
    Stdio,
    /// HTTP server
    Http { host: String, port: u16 },
    /// WebSocket
    WebSocket { url: String },
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            name: "rustforge-mcp".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            transport: TransportType::Stdio,
            max_concurrent: 10,
            timeout_secs: 30,
        }
    }
}

impl McpConfig {
    /// Create a new configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Set the version
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Use stdio transport
    pub fn stdio(mut self) -> Self {
        self.transport = TransportType::Stdio;
        self
    }

    /// Use HTTP transport
    pub fn http(mut self, host: &str, port: u16) -> Self {
        self.transport = TransportType::Http {
            host: host.to_string(),
            port,
        };
        self
    }

    /// Use WebSocket transport
    pub fn websocket(mut self, url: &str) -> Self {
        self.transport = TransportType::WebSocket {
            url: url.to_string(),
        };
        self
    }

    /// Set maximum concurrent requests
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Set the global configuration
pub fn set_config(config: McpConfig) -> &'static McpConfig {
    CONFIG.get_or_init(|| config)
}

/// Get the global configuration
pub fn get_config() -> &'static McpConfig {
    CONFIG.get_or_init(McpConfig::default)
}
