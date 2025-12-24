//! rf-mcp - Laravel MCP-style AI integration for RustForge
//!
//! Model Context Protocol (MCP) integration for connecting AI assistants
//! to external data sources and tools.
//!
//! # Example
//!
//! ```rust,ignore
//! use rf_mcp::{Mcp, Tool, Resource, Prompt};
//!
//! // Register a tool
//! Mcp::tool("search_users", |input: SearchInput| async {
//!     // Search users logic
//!     Ok(SearchResult { users: vec![] })
//! });
//!
//! // Register a resource
//! Mcp::resource("user://{id}", |uri| async {
//!     // Fetch user resource
//!     Ok(Resource::text("User data..."))
//! });
//!
//! // Register a prompt template
//! Mcp::prompt("summarize", |args| {
//!     Prompt::new("Summarize the following: {content}")
//!         .with_arg("content", args.get("content"))
//! });
//!
//! // Start MCP server
//! Mcp::serve().await;
//! ```

mod config;
mod errors;
mod prompt;
mod resource;
mod server;
mod tool;
mod transport;

pub use config::{get_config, set_config, McpConfig};
pub use errors::{McpError, McpResult};
pub use prompt::{Prompt, PromptBuilder, PromptMessage, PromptRegistry};
pub use resource::{Resource, ResourceBuilder, ResourceContent, ResourceRegistry};
pub use server::McpServer;
pub use tool::{Tool, ToolBuilder, ToolInput, ToolRegistry, ToolResult};
pub use transport::{StdioTransport, Transport};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// MCP facade - Laravel-style static interface
pub struct Mcp;

impl Mcp {
    /// Create a tool builder
    pub fn tool(name: &str) -> SimpleToolBuilder {
        SimpleToolBuilder::new(name)
    }

    /// Register a resource with the MCP server
    pub fn resource(uri_template: &str) -> ResourceBuilder {
        ResourceBuilder::new(uri_template)
    }

    /// Register a prompt template
    pub fn prompt(name: &str, template: &str) -> PromptBuilder {
        PromptBuilder::new(name, template)
    }

    /// Start the MCP server
    pub async fn serve() -> McpResult<()> {
        let server = McpServer::new();
        server.run().await
    }

    /// Create a new MCP server instance
    pub fn server() -> McpServer {
        McpServer::new()
    }

    /// Get the tool registry
    pub fn tools() -> Arc<ToolRegistry> {
        ToolRegistry::global()
    }

    /// Get the resource registry
    pub fn resources() -> Arc<ResourceRegistry> {
        ResourceRegistry::global()
    }

    /// Get the prompt registry
    pub fn prompts() -> Arc<PromptRegistry> {
        PromptRegistry::global()
    }
}

/// Simple tool builder for creating tools
pub struct SimpleToolBuilder {
    name: String,
    description: Option<String>,
}

impl SimpleToolBuilder {
    /// Create a new simple tool builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
        }
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the handler and get a full tool builder
    pub fn handler<F>(self, handler: F) -> ToolBuilder
    where
        F: Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = McpResult<serde_json::Value>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let mut builder = ToolBuilder::new(&self.name, handler);
        if let Some(desc) = self.description {
            builder = builder.description(&desc);
        }
        builder
    }
}
