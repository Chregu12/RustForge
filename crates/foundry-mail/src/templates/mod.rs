pub mod engine;
pub mod renderer;
pub mod tera_renderer;
pub mod handlebars_renderer;

pub use engine::{TemplateEngine, TemplateEngineError};
pub use renderer::{TemplateRenderer, RenderContext};
pub use tera_renderer::TeraRenderer;
pub use handlebars_renderer::HandlebarsRenderer;

use std::collections::HashMap;
use serde::Serialize;

/// Template data for rendering
pub type TemplateData = HashMap<String, serde_json::Value>;

/// Create template data from a serializable value
pub fn template_data<T: Serialize>(value: &T) -> Result<TemplateData, serde_json::Error> {
    let json = serde_json::to_value(value)?;
    if let serde_json::Value::Object(map) = json {
        Ok(map.into_iter().collect())
    } else {
        Ok(HashMap::new())
    }
}
