//! Template-based export rendering

use serde::Serialize;
use tera::{Context, Tera};

/// Template engine for export documents
pub struct TemplateEngine {
    tera: Tera,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            tera: Tera::default(),
        }
    }

    pub fn register_template(&mut self, name: &str, content: &str) -> anyhow::Result<()> {
        self.tera.add_raw_template(name, content)?;
        Ok(())
    }

    pub fn render<T: Serialize>(&self, template: &str, context: &T) -> anyhow::Result<String> {
        let ctx = Context::from_serialize(context)?;
        Ok(self.tera.render(template, &ctx)?)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for rendering templates
pub trait TemplateRenderer {
    fn render(&self, engine: &TemplateEngine) -> anyhow::Result<String>;
}
