//! Class-Based Components
//!
//! Class components for more complex, stateful component logic

use super::attributes::AttributeBag;
use super::props::ComponentProps;
use crate::compiler_new::{Compiler, RenderContext};
use crate::parser_new::Parser;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComponentError {
    #[error("Component render error: {0}")]
    RenderError(String),

    #[error("Template parse error: {0}")]
    ParseError(String),

    #[error("Prop error: {0}")]
    PropError(#[from] super::props::PropError),
}

pub type ComponentResult<T> = Result<T, ComponentError>;

/// Trait for class-based components
///
/// Implement this trait to create reusable, stateful components
pub trait Component: Send + Sync + std::fmt::Debug {
    /// Render the component
    ///
    /// # Arguments
    /// * `props` - Component properties
    /// * `attributes` - HTML attributes
    /// * `slots` - Named slots content
    ///
    /// # Returns
    /// Rendered HTML string
    fn render(
        &self,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> ComponentResult<String>;

    /// Component name
    fn name(&self) -> &str;

    /// Get component data
    ///
    /// Override to provide additional data to the template
    fn data(&self) -> Value {
        serde_json::json!({})
    }

    /// Validate props before rendering
    ///
    /// Override to add custom validation logic
    fn validate_props(&self, _props: &ComponentProps) -> ComponentResult<()> {
        Ok(())
    }

    /// Lifecycle hook called before render
    fn before_render(&self, _props: &ComponentProps) -> ComponentResult<()> {
        Ok(())
    }

    /// Lifecycle hook called after render
    fn after_render(&self, _html: &str) -> ComponentResult<String> {
        Ok(_html.to_string())
    }
}

/// Base component implementation
///
/// Provides common functionality for template-based components
#[derive(Debug)]
pub struct BaseComponent {
    name: String,
    template: String,
}

impl BaseComponent {
    /// Create a new base component
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
        }
    }

    /// Render the component template with context
    pub fn render_template(
        &self,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> ComponentResult<String> {
        // Build context data
        let mut data = serde_json::Map::new();

        // Add props
        for (key, value) in props.all() {
            data.insert(key, value);
        }

        // Add attributes
        data.insert("attributes".to_string(), serde_json::json!({
            "class": attributes.get("class").unwrap_or(&String::new()),
            "all": attributes.to_html(),
        }));

        // Add slots
        let default_slot = slots.get("default").cloned().unwrap_or_default();
        data.insert("slot".to_string(), serde_json::json!(default_slot));

        // Add named slots
        let mut slots_map = serde_json::Map::new();
        for (name, content) in slots {
            if name != "default" {
                slots_map.insert(name.clone(), serde_json::json!(content));
            }
        }
        data.insert("slots".to_string(), serde_json::json!(slots_map));

        // Parse and compile template
        let ast = Parser::parse(&self.template)
            .map_err(|e| ComponentError::ParseError(e.to_string()))?;

        let mut context = RenderContext::new(Value::Object(data));
        let compiler = Compiler::new();

        compiler
            .compile(&ast, &mut context)
            .map_err(|e| ComponentError::RenderError(e.to_string()))
    }
}

impl Component for BaseComponent {
    fn render(
        &self,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> ComponentResult<String> {
        self.render_template(props, attributes, slots)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_component() {
        let component = BaseComponent::new(
            "alert",
            r#"<div class="alert">{{ $slot }}</div>"#,
        );

        assert_eq!(component.name(), "alert");
    }

    #[test]
    fn test_render_component() {
        let component = BaseComponent::new(
            "alert",
            r#"<div class="alert alert-{{ $type }}">{{ $slot }}</div>"#,
        );

        let mut props = ComponentProps::new();
        props.set("type".to_string(), serde_json::json!("danger"));

        let attributes = AttributeBag::new();

        let mut slots = HashMap::new();
        slots.insert("default".to_string(), "Error message!".to_string());

        let html = component.render(&props, &attributes, &slots).unwrap();

        assert!(html.contains("alert-danger"));
        assert!(html.contains("Error message!"));
    }

    #[test]
    fn test_render_with_attributes() {
        let component = BaseComponent::new(
            "button",
            r#"<button class="{{ $attributes.class }}">{{ $slot }}</button>"#,
        );

        let props = ComponentProps::new();

        let mut attributes = AttributeBag::new();
        attributes.set("class".to_string(), "btn btn-primary".to_string());

        let mut slots = HashMap::new();
        slots.insert("default".to_string(), "Click me".to_string());

        let html = component.render(&props, &attributes, &slots).unwrap();

        assert!(html.contains("btn btn-primary"));
        assert!(html.contains("Click me"));
    }

    #[test]
    fn test_render_with_named_slots() {
        let component = BaseComponent::new(
            "card",
            r#"<div class="card"><div class="card-header">{{ $slots.header }}</div><div class="card-body">{{ $slot }}</div></div>"#,
        );

        let props = ComponentProps::new();
        let attributes = AttributeBag::new();

        let mut slots = HashMap::new();
        slots.insert("default".to_string(), "Body content".to_string());
        slots.insert("header".to_string(), "Card Title".to_string());

        let html = component.render(&props, &attributes, &slots).unwrap();

        assert!(html.contains("Card Title"));
        assert!(html.contains("Body content"));
    }
}
