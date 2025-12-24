//! MCP Resource definitions

use crate::errors::{McpError, McpResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock, RwLock};

type ResourceHandler = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = McpResult<ResourceContent>> + Send>>
        + Send
        + Sync,
>;

static RESOURCE_REGISTRY: OnceLock<Arc<ResourceRegistry>> = OnceLock::new();

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    /// URI of the resource
    pub uri: String,
    /// MIME type
    pub mime_type: String,
    /// Text content (for text resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Binary content (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl ResourceContent {
    /// Create a text resource
    pub fn text(uri: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: "text/plain".to_string(),
            text: Some(content.into()),
            blob: None,
        }
    }

    /// Create a JSON resource
    pub fn json(uri: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: "application/json".to_string(),
            text: Some(content.into()),
            blob: None,
        }
    }

    /// Create an HTML resource
    pub fn html(uri: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: "text/html".to_string(),
            text: Some(content.into()),
            blob: None,
        }
    }

    /// Create a markdown resource
    pub fn markdown(uri: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type: "text/markdown".to_string(),
            text: Some(content.into()),
            blob: None,
        }
    }

    /// Create a binary resource
    pub fn binary(uri: impl Into<String>, mime_type: impl Into<String>, data: &[u8]) -> Self {
        use base64::Engine;
        Self {
            uri: uri.into(),
            mime_type: mime_type.into(),
            text: None,
            blob: Some(base64::engine::general_purpose::STANDARD.encode(data)),
        }
    }
}

/// A registered resource
#[derive(Clone)]
pub struct Resource {
    /// URI template (e.g., "user://{id}")
    pub uri_template: String,
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: Option<String>,
    /// MIME type
    pub mime_type: String,
    /// Handler function
    handler: ResourceHandler,
}

impl Resource {
    /// Fetch the resource
    pub async fn fetch(&self, uri: &str) -> McpResult<ResourceContent> {
        (self.handler)(uri.to_string()).await
    }

    /// Check if this resource matches a URI
    pub fn matches(&self, uri: &str) -> bool {
        // Simple template matching (e.g., "user://{id}" matches "user://123")
        let template_parts: Vec<&str> = self.uri_template.split("://").collect();
        let uri_parts: Vec<&str> = uri.split("://").collect();

        if template_parts.len() != uri_parts.len() {
            return false;
        }

        // Check scheme matches
        if template_parts.first() != uri_parts.first() {
            return false;
        }

        true // Simple match for now
    }
}

/// Resource builder
pub struct ResourceBuilder {
    uri_template: String,
    name: Option<String>,
    description: Option<String>,
    mime_type: String,
    handler: Option<ResourceHandler>,
}

impl ResourceBuilder {
    /// Create a new resource builder
    pub fn new(uri_template: &str) -> Self {
        // Extract name from template
        let name = uri_template
            .split("://")
            .next()
            .unwrap_or("resource")
            .to_string();

        Self {
            uri_template: uri_template.to_string(),
            name: Some(name),
            description: None,
            mime_type: "text/plain".to_string(),
            handler: None,
        }
    }

    /// Set the name
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set the description
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the MIME type
    pub fn mime_type(mut self, mime_type: &str) -> Self {
        self.mime_type = mime_type.to_string();
        self
    }

    /// Set the handler
    pub fn handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = McpResult<ResourceContent>> + Send + 'static,
    {
        self.handler = Some(Arc::new(move |uri| {
            Box::pin(handler(uri)) as Pin<Box<dyn Future<Output = McpResult<ResourceContent>> + Send>>
        }));
        self
    }

    /// Register the resource
    pub fn register(self) {
        if let Some(handler) = self.handler {
            let resource = Resource {
                uri_template: self.uri_template.clone(),
                name: self.name.unwrap_or_else(|| "resource".to_string()),
                description: self.description,
                mime_type: self.mime_type,
                handler,
            };

            ResourceRegistry::global().register(resource);
        }
    }
}

/// Resource registry
pub struct ResourceRegistry {
    resources: RwLock<HashMap<String, Resource>>,
}

impl ResourceRegistry {
    /// Create a new resource registry
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global resource registry
    pub fn global() -> Arc<Self> {
        RESOURCE_REGISTRY
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    /// Register a resource
    pub fn register(&self, resource: Resource) {
        let mut resources = self.resources.write().unwrap();
        resources.insert(resource.uri_template.clone(), resource);
    }

    /// Find a resource by URI
    pub fn find(&self, uri: &str) -> Option<Resource> {
        let resources = self.resources.read().unwrap();
        resources.values().find(|r| r.matches(uri)).cloned()
    }

    /// List all resources
    pub fn list(&self) -> Vec<Resource> {
        let resources = self.resources.read().unwrap();
        resources.values().cloned().collect()
    }

    /// Fetch a resource
    pub async fn fetch(&self, uri: &str) -> McpResult<ResourceContent> {
        let resource = self
            .find(uri)
            .ok_or_else(|| McpError::ResourceNotFound(uri.to_string()))?;
        resource.fetch(uri).await
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
