use super::{TemplateEngine, TemplateEngineError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tera::Tera;

/// Tera template renderer
pub struct TeraRenderer {
    tera: Tera,
    template_dir: PathBuf,
}

impl TeraRenderer {
    pub fn new(template_dir: impl Into<PathBuf>) -> Result<Self, TemplateEngineError> {
        let template_dir = template_dir.into();
        let pattern = template_dir.join("**/*.html");
        let pattern_str = pattern.to_str()
            .ok_or_else(|| TemplateEngineError::Template("Invalid template directory path".into()))?;

        let tera = Tera::new(pattern_str)
            .map_err(|e| TemplateEngineError::Template(e.to_string()))?;

        Ok(Self { tera, template_dir })
    }

    pub fn from_string() -> Self {
        Self {
            tera: Tera::default(),
            template_dir: PathBuf::new(),
        }
    }
}

#[async_trait]
impl TemplateEngine for TeraRenderer {
    async fn render(&self, template: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError> {
        let mut ctx = tera::Context::new();
        for (key, value) in context {
            ctx.insert(key, value);
        }

        let mut tera = self.tera.clone();
        tera.add_raw_template("inline", template)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))?;

        tera.render("inline", &ctx)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))
    }

    async fn render_file(&self, path: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError> {
        let mut ctx = tera::Context::new();
        for (key, value) in context {
            ctx.insert(key, value);
        }

        self.tera
            .render(path, &ctx)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tera_render() {
        let renderer = TeraRenderer::from_string();
        let template = "Hello {{ name }}!";
        let mut context = HashMap::new();
        context.insert("name".to_string(), serde_json::Value::String("World".to_string()));

        let result = renderer.render(template, &context).await.unwrap();
        assert_eq!(result, "Hello World!");
    }
}
