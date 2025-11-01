use super::{TemplateEngine, TemplateEngineError};
use async_trait::async_trait;
use handlebars::Handlebars;
use std::collections::HashMap;
use std::path::PathBuf;

/// Handlebars template renderer
pub struct HandlebarsRenderer {
    handlebars: Handlebars<'static>,
    template_dir: PathBuf,
}

impl HandlebarsRenderer {
    pub fn new(template_dir: impl Into<PathBuf>) -> Result<Self, TemplateEngineError> {
        let template_dir = template_dir.into();
        let mut handlebars = Handlebars::new();

        // Register templates from directory
        if template_dir.exists() {
            Self::register_templates_from_dir(&mut handlebars, &template_dir)?;
        }

        Ok(Self {
            handlebars,
            template_dir,
        })
    }

    pub fn from_string() -> Self {
        Self {
            handlebars: Handlebars::new(),
            template_dir: PathBuf::new(),
        }
    }

    fn register_templates_from_dir(
        handlebars: &mut Handlebars<'static>,
        dir: &PathBuf,
    ) -> Result<(), TemplateEngineError> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "hbs" || ext == "handlebars" {
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            handlebars
                                .register_template_file(name, &path)
                                .map_err(|e| TemplateEngineError::Template(e.to_string()))?;
                        }
                    }
                }
            } else if path.is_dir() {
                Self::register_templates_from_dir(handlebars, &path)?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl TemplateEngine for HandlebarsRenderer {
    async fn render(&self, template: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError> {
        let data = serde_json::to_value(context)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))?;

        self.handlebars
            .render_template(template, &data)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))
    }

    async fn render_file(&self, path: &str, context: &HashMap<String, serde_json::Value>) -> Result<String, TemplateEngineError> {
        let data = serde_json::to_value(context)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))?;

        self.handlebars
            .render(path, &data)
            .map_err(|e| TemplateEngineError::RenderError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handlebars_render() {
        let renderer = HandlebarsRenderer::from_string();
        let template = "Hello {{name}}!";
        let mut context = HashMap::new();
        context.insert("name".to_string(), serde_json::Value::String("World".to_string()));

        let result = renderer.render(template, &context).await.unwrap();
        assert_eq!(result, "Hello World!");
    }
}
