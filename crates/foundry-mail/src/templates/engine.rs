use async_trait::async_trait;
use std::collections::HashMap;

/// Template engine trait
#[async_trait]
pub trait TemplateEngine: Send + Sync {
    /// Render a template with the given context
    async fn render(&self, template: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError>;

    /// Render a template from file
    async fn render_file(&self, path: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateEngineError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Template render error: {0}")]
    RenderError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Template error: {0}")]
    Template(String),
}
