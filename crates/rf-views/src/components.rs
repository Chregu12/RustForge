use crate::{context::Context, error::ViewResult, ViewError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tera::{Function, Result as TeraResult};

/// Escape HTML special characters to prevent XSS
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Type alias for component renderer function
pub type ComponentRenderer = Arc<dyn Fn(&Context) -> ViewResult<String> + Send + Sync>;

/// Component registry for managing reusable view components
#[derive(Clone)]
pub struct ComponentRegistry {
    components: Arc<RwLock<HashMap<String, ComponentRenderer>>>,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            components: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a component
    pub fn register<F>(&self, name: impl Into<String>, renderer: F) -> ViewResult<()>
    where
        F: Fn(&Context) -> ViewResult<String> + Send + Sync + 'static,
    {
        let mut components = self
            .components
            .write()
            .map_err(|e| ViewError::ComponentError(format!("Failed to acquire lock: {}", e)))?;

        components.insert(name.into(), Arc::new(renderer));
        Ok(())
    }

    /// Render a component
    pub fn render(&self, name: &str, context: &Context) -> ViewResult<String> {
        let components = self
            .components
            .read()
            .map_err(|e| ViewError::ComponentError(format!("Failed to acquire lock: {}", e)))?;

        let renderer = components
            .get(name)
            .ok_or_else(|| ViewError::ComponentError(format!("Component not found: {}", name)))?;

        renderer(context)
    }

    /// Check if a component exists
    pub fn has(&self, name: &str) -> bool {
        if let Ok(components) = self.components.read() {
            components.contains_key(name)
        } else {
            false
        }
    }

    /// Remove a component
    pub fn unregister(&self, name: &str) -> ViewResult<()> {
        let mut components = self
            .components
            .write()
            .map_err(|e| ViewError::ComponentError(format!("Failed to acquire lock: {}", e)))?;

        components.remove(name);
        Ok(())
    }

    /// Get all registered component names
    pub fn component_names(&self) -> Vec<String> {
        if let Ok(components) = self.components.read() {
            components.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tera function for rendering components
#[derive(Clone)]
pub struct ComponentFunction {
    registry: ComponentRegistry,
}

impl ComponentFunction {
    /// Create a new component function
    pub fn new(registry: ComponentRegistry) -> Self {
        Self { registry }
    }
}

impl Function for ComponentFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("Component 'name' parameter is required"))?;

        // Build context from remaining args
        let mut context = Context::new();
        for (key, value) in args {
            if key != "name" {
                context.insert(key, value.clone());
            }
        }

        let html = self
            .registry
            .render(name, &context)
            .map_err(|e| tera::Error::msg(format!("Component error: {}", e)))?;

        Ok(Value::String(html))
    }
}

/// Pre-built alert component
pub fn alert_component() -> ComponentRenderer {
    Arc::new(|context: &Context| {
        let alert_type = context
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        let message = context
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ViewError::ComponentError("Alert message is required".to_string()))?;

        let dismissible = context
            .get("dismissible")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut html = format!(r#"<div class="alert alert-{}" role="alert">"#, escape_html(alert_type));

        if dismissible {
            html.push_str(
                r#"<button type="button" class="close" data-dismiss="alert">&times;</button>"#,
            );
        }

        html.push_str(&format!("{}</div>", escape_html(message)));

        Ok(html)
    })
}

/// Pre-built card component
pub fn card_component() -> ComponentRenderer {
    Arc::new(|context: &Context| {
        let title = context.get("title").and_then(|v| v.as_str());

        let content = context
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ViewError::ComponentError("Card content is required".to_string()))?;

        let footer = context.get("footer").and_then(|v| v.as_str());

        let mut html = String::from(r#"<div class="card">"#);

        if let Some(title) = title {
            html.push_str(&format!(
                r#"<div class="card-header"><h3>{}</h3></div>"#,
                escape_html(title)
            ));
        }

        html.push_str(&format!(r#"<div class="card-body">{}</div>"#, escape_html(content)));

        if let Some(footer) = footer {
            html.push_str(&format!(r#"<div class="card-footer">{}</div>"#, escape_html(footer)));
        }

        html.push_str("</div>");

        Ok(html)
    })
}

/// Pre-built button component
pub fn button_component() -> ComponentRenderer {
    Arc::new(|context: &Context| {
        let text = context
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ViewError::ComponentError("Button text is required".to_string()))?;

        let button_type = context
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("button");

        let variant = context
            .get("variant")
            .and_then(|v| v.as_str())
            .unwrap_or("primary");

        let disabled = context
            .get("disabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut html = format!(
            r#"<button type="{}" class="btn btn-{}"#,
            button_type, variant
        );

        if disabled {
            html.push_str(r#" disabled"#);
        }

        html.push_str(&format!(r#">{}</button>"#, text));

        Ok(html)
    })
}

/// Pre-built form input component
pub fn input_component() -> ComponentRenderer {
    Arc::new(|context: &Context| {
        let name = context
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ViewError::ComponentError("Input name is required".to_string()))?;

        let input_type = context
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        let label = context.get("label").and_then(|v| v.as_str());

        let value = context.get("value").and_then(|v| v.as_str()).unwrap_or("");

        let placeholder = context.get("placeholder").and_then(|v| v.as_str());

        let required = context
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let error = context.get("error").and_then(|v| v.as_str());

        let mut html = String::from(r#"<div class="form-group">"#);

        if let Some(label_text) = label {
            html.push_str(&format!(r#"<label for="{}">{}</label>"#, escape_html(name), escape_html(label_text)));
        }

        html.push_str(&format!(
            r#"<input type="{}" name="{}" id="{}" class="form-control"#,
            escape_html(input_type), escape_html(name), escape_html(name)
        ));

        if !value.is_empty() {
            html.push_str(&format!(r#" value="{}""#, escape_html(value)));
        }

        if let Some(ph) = placeholder {
            html.push_str(&format!(r#" placeholder="{}""#, escape_html(ph)));
        }

        if required {
            html.push_str(r#" required"#);
        }

        html.push('>');

        if let Some(err) = error {
            html.push_str(&format!(r#"<span class="error-message">{}</span>"#, escape_html(err)));
        }

        html.push_str("</div>");

        Ok(html)
    })
}

/// Register default components
pub fn register_default_components(registry: &ComponentRegistry) -> ViewResult<()> {
    registry.register("alert", |ctx: &Context| alert_component()(ctx))?;
    registry.register("card", |ctx: &Context| card_component()(ctx))?;
    registry.register("button", |ctx: &Context| button_component()(ctx))?;
    registry.register("input", |ctx: &Context| input_component()(ctx))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context;

    #[test]
    fn test_component_registry() {
        let registry = ComponentRegistry::new();

        registry
            .register("test", |ctx: &Context| {
                let name = ctx.get("name").and_then(|v| v.as_str()).unwrap_or("World");
                Ok(format!("Hello, {}!", name))
            })
            .unwrap();

        assert!(registry.has("test"));

        let ctx = context! { "name" => "Rust" };
        let result = registry.render("test", &ctx).unwrap();
        assert_eq!(result, "Hello, Rust!");
    }

    #[test]
    fn test_alert_component() {
        let alert = alert_component();

        let ctx = context! {
            "type" => "success",
            "message" => "Operation completed!",
        };

        let result = alert(&ctx).unwrap();
        assert!(result.contains("alert-success"));
        assert!(result.contains("Operation completed!"));
    }

    #[test]
    fn test_card_component() {
        let card = card_component();

        let ctx = context! {
            "title" => "Card Title",
            "content" => "Card content goes here",
            "footer" => "Card footer",
        };

        let result = card(&ctx).unwrap();
        assert!(result.contains("Card Title"));
        assert!(result.contains("Card content goes here"));
        assert!(result.contains("Card footer"));
    }

    #[test]
    fn test_button_component() {
        let button = button_component();

        let ctx = context! {
            "text" => "Click me",
            "variant" => "danger",
        };

        let result = button(&ctx).unwrap();
        assert!(result.contains("Click me"));
        assert!(result.contains("btn-danger"));
    }

    #[test]
    fn test_input_component() {
        let input = input_component();

        let ctx = context! {
            "name" => "email",
            "type" => "email",
            "label" => "Email Address",
            "placeholder" => "Enter your email",
            "required" => true,
        };

        let result = input(&ctx).unwrap();
        assert!(result.contains("Email Address"));
        assert!(result.contains("type=\"email\""));
        assert!(result.contains("required"));
    }

    #[test]
    fn test_register_default_components() {
        let registry = ComponentRegistry::new();
        register_default_components(&registry).unwrap();

        assert!(registry.has("alert"));
        assert!(registry.has("card"));
        assert!(registry.has("button"));
        assert!(registry.has("input"));
    }
}
