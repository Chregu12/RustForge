use super::{TemplateData, TemplateEngine, TemplateEngineError};
use std::sync::Arc;

/// Template rendering context
pub struct RenderContext {
    pub data: TemplateData,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            data: TemplateData::new(),
        }
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.data.insert(key.into(), value.into());
        self
    }

    pub fn merge(mut self, data: TemplateData) -> Self {
        self.data.extend(data);
        self
    }
}

impl Default for RenderContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Template renderer
pub struct TemplateRenderer {
    engine: Arc<dyn TemplateEngine>,
}

impl TemplateRenderer {
    pub fn new(engine: Arc<dyn TemplateEngine>) -> Self {
        Self { engine }
    }

    pub async fn render(&self, template: &str, context: &RenderContext) -> Result<String, TemplateEngineError> {
        self.engine.render(template, &context.data).await
    }

    pub async fn render_file(&self, path: &str, context: &RenderContext) -> Result<String, TemplateEngineError> {
        self.engine.render_file(path, &context.data).await
    }
}
