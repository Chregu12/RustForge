//! Component Compiler Integration
//!
//! Pre-processes templates to compile <x-*> component tags

use super::parser::{ComponentParser, ParseError};
use super::registry::ComponentRegistry;
use super::{AttributeBag, ComponentProps, SlotBag};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ComponentCompileError {
    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Compilation error: {0}")]
    CompilationError(String),
}

pub type ComponentCompileResult<T> = Result<T, ComponentCompileError>;

/// Component compiler that processes <x-*> tags in templates
pub struct ComponentCompiler {
    parser: ComponentParser,
    registry: Arc<ComponentRegistry>,
}

impl ComponentCompiler {
    /// Create a new component compiler
    pub fn new(registry: Arc<ComponentRegistry>) -> ComponentCompileResult<Self> {
        Ok(Self {
            parser: ComponentParser::new()?,
            registry,
        })
    }

    /// Compile a template, replacing all component tags with rendered output
    ///
    /// # Example
    ///
    /// ```ignore
    /// let compiler = ComponentCompiler::new(registry)?;
    /// let compiled = compiler.compile(r#"
    ///     <x-alert type="danger">Error message</x-alert>
    /// "#)?;
    /// ```
    pub fn compile(&self, template: &str) -> ComponentCompileResult<String> {
        let mut result = template.to_string();

        // Parse all component tags
        let tags = self.parser.parse_all(template)?;

        // Process tags in reverse order to handle nested components correctly
        // This prevents offset issues when replacing text
        let mut tags_sorted: Vec<_> = tags.into_iter().collect();
        tags_sorted.reverse();

        for tag in tags_sorted {
            // Build props from attributes
            let mut props = ComponentProps::new();
            for (key, value) in &tag.attributes {
                // Remove {{ }} markers if present (from bound attributes)
                let clean_value = value
                    .strip_prefix("{{ ")
                    .and_then(|s| s.strip_suffix(" }}"))
                    .unwrap_or(value);

                props.set(key.clone(), serde_json::json!(clean_value));
            }

            // Build attribute bag
            let attr_bag = AttributeBag::from_pairs(
                tag.attributes
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );

            // Convert SlotBag to HashMap for compatibility
            let slots = tag.slots.to_map();

            // Render component
            let rendered = self
                .registry
                .render_component(&tag.name, &props, &attr_bag, &slots)
                .map_err(|e| ComponentCompileError::RegistryError(e.to_string()))?;

            // Replace in template
            result = result.replace(&tag.raw, &rendered);
        }

        Ok(result)
    }

    /// Check if template contains components
    pub fn has_components(&self, template: &str) -> bool {
        self.parser.has_components(template)
    }

    /// Get component tag positions for debugging
    pub fn find_components(&self, template: &str) -> Vec<String> {
        self.parser.find_component_tags(template)
    }
}

/// Builder for component compiler
pub struct ComponentCompilerBuilder {
    registry: Option<Arc<ComponentRegistry>>,
}

impl ComponentCompilerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { registry: None }
    }

    /// Set component registry
    pub fn with_registry(mut self, registry: Arc<ComponentRegistry>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Build the component compiler
    pub fn build(self) -> ComponentCompileResult<ComponentCompiler> {
        let registry = self.registry.ok_or_else(|| {
            ComponentCompileError::CompilationError("Registry not set".to_string())
        })?;

        ComponentCompiler::new(registry)
    }
}

impl Default for ComponentCompilerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::class_component::BaseComponent;

    fn create_test_registry() -> Arc<ComponentRegistry> {
        let mut registry = ComponentRegistry::new();

        // Register an alert component
        let alert = BaseComponent::new(
            "alert",
            r#"<div class="alert alert-{{ $type }}">{{ $slot }}</div>"#,
        );
        registry.register("alert", alert).unwrap();

        // Register a card component with slots
        let card = BaseComponent::new(
            "card",
            r#"<div class="card">
                <div class="card-header">{{ $slots.header }}</div>
                <div class="card-body">{{ $slot }}</div>
                <div class="card-footer">{{ $slots.footer }}</div>
            </div>"#,
        );
        registry.register("card", card).unwrap();

        Arc::new(registry)
    }

    #[test]
    fn test_compile_simple_component() {
        let registry = create_test_registry();
        let compiler = ComponentCompiler::new(registry).unwrap();

        let template = r#"<x-alert type="danger">Error message</x-alert>"#;
        let result = compiler.compile(template).unwrap();

        assert!(result.contains("alert-danger"));
        assert!(result.contains("Error message"));
    }

    #[test]
    fn test_compile_component_with_slots() {
        let registry = create_test_registry();
        let compiler = ComponentCompiler::new(registry).unwrap();

        let template = r#"
            <x-card>
                <x-slot name="header">Card Title</x-slot>
                <x-slot name="footer">Card Footer</x-slot>
                Card Body
            </x-card>
        "#;

        let result = compiler.compile(template).unwrap();

        assert!(result.contains("Card Title"));
        assert!(result.contains("Card Body"));
        assert!(result.contains("Card Footer"));
    }

    #[test]
    fn test_compile_multiple_components() {
        let registry = create_test_registry();
        let compiler = ComponentCompiler::new(registry).unwrap();

        let template = r#"
            <x-alert type="info">Info message</x-alert>
            <x-alert type="danger">Error message</x-alert>
        "#;

        let result = compiler.compile(template).unwrap();

        assert!(result.contains("alert-info"));
        assert!(result.contains("alert-danger"));
    }

    #[test]
    fn test_has_components() {
        let registry = create_test_registry();
        let compiler = ComponentCompiler::new(registry).unwrap();

        assert!(compiler.has_components("<x-alert>Test</x-alert>"));
        assert!(!compiler.has_components("<div>No components</div>"));
    }

    #[test]
    fn test_find_components() {
        let registry = create_test_registry();
        let compiler = ComponentCompiler::new(registry).unwrap();

        let template = r#"
            <x-alert type="info">Test</x-alert>
            <x-card>Body</x-card>
        "#;

        let components = compiler.find_components(template);
        assert_eq!(components.len(), 2);
    }

    #[test]
    fn test_compiler_builder() {
        let registry = create_test_registry();

        let compiler = ComponentCompilerBuilder::new()
            .with_registry(registry)
            .build()
            .unwrap();

        assert!(compiler.has_components("<x-alert>Test</x-alert>"));
    }
}
