//! Component Registry
//!
//! Manages component registration and resolution

use super::attributes::AttributeBag;
use super::class_component::Component;
use super::props::ComponentProps;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Component already registered: {0}")]
    ComponentAlreadyRegistered(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Render error: {0}")]
    RenderError(String),
}

pub type RegistryResult<T> = Result<T, RegistryError>;

/// Component registry for managing components
#[derive(Debug)]
pub struct ComponentRegistry {
    /// Registered class-based components
    components: HashMap<String, Arc<dyn Component>>,

    /// Anonymous component template paths
    anonymous: HashMap<String, PathBuf>,

    /// Component search paths
    search_paths: Vec<PathBuf>,
}

impl ComponentRegistry {
    /// Create a new component registry
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            anonymous: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// Register a class-based component
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// registry.register("alert", AlertComponent::new());
    /// ```
    pub fn register<C: Component + 'static>(
        &mut self,
        name: impl Into<String>,
        component: C,
    ) -> RegistryResult<()> {
        let name = name.into();

        if self.components.contains_key(&name) {
            return Err(RegistryError::ComponentAlreadyRegistered(name));
        }

        self.components.insert(name, Arc::new(component));
        Ok(())
    }

    /// Register a component with an alias
    pub fn alias(
        &mut self,
        original: impl AsRef<str>,
        alias: impl Into<String>,
    ) -> RegistryResult<()> {
        let original = original.as_ref();
        let alias = alias.into();

        if let Some(component) = self.components.get(original) {
            self.components.insert(alias, Arc::clone(component));
            Ok(())
        } else {
            Err(RegistryError::ComponentNotFound(original.to_string()))
        }
    }

    /// Add component search path for anonymous components
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// Discover anonymous components from filesystem
    ///
    /// Scans search paths for .blade.php or .blade.html files
    /// and registers them as anonymous components
    pub fn discover_anonymous(&mut self, base_path: impl AsRef<Path>) -> RegistryResult<()> {
        let base_path = base_path.as_ref();

        if !base_path.exists() {
            return Ok(()); // Silently skip non-existent paths
        }

        self.scan_directory(base_path, base_path)?;

        Ok(())
    }

    /// Recursively scan directory for component templates
    fn scan_directory(&mut self, base_path: &Path, current_path: &Path) -> RegistryResult<()> {
        if !current_path.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(base_path, &path)?;
            } else if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();

                if file_name_str.ends_with(".blade.html") || file_name_str.ends_with(".blade.php") {
                    // Extract component name from path relative to base
                    if let Ok(relative_path) = path.strip_prefix(base_path) {
                        let component_name = relative_path
                            .with_extension("")
                            .with_extension("")
                            .to_string_lossy()
                            .replace(std::path::MAIN_SEPARATOR, ".");

                        self.anonymous.insert(component_name, path.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if component exists (class-based or anonymous)
    pub fn has(&self, name: &str) -> bool {
        self.components.contains_key(name) || self.anonymous.contains_key(name)
    }

    /// Resolve component template path (for anonymous components)
    pub fn resolve_template(&self, name: &str) -> Option<&PathBuf> {
        self.anonymous.get(name)
    }

    /// Render a component
    ///
    /// Handles both class-based and anonymous components
    pub fn render_component(
        &self,
        name: &str,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> RegistryResult<String> {
        // Try class-based component first
        if let Some(component) = self.components.get(name) {
            return component
                .render(props, attributes, slots)
                .map_err(|e| RegistryError::RenderError(e.to_string()));
        }

        // Try anonymous component
        if let Some(template_path) = self.anonymous.get(name) {
            return self.render_anonymous(template_path, props, attributes, slots);
        }

        Err(RegistryError::ComponentNotFound(name.to_string()))
    }

    /// Render an anonymous component from file
    fn render_anonymous(
        &self,
        template_path: &Path,
        props: &ComponentProps,
        attributes: &AttributeBag,
        slots: &HashMap<String, String>,
    ) -> RegistryResult<String> {
        use crate::compiler_new::{Compiler, RenderContext};
        use crate::parser_new::Parser;
        use serde_json::Value;

        // Load template
        let template = std::fs::read_to_string(template_path)?;

        // Build context data
        let mut data = serde_json::Map::new();

        // Add props
        for (key, value) in props.all() {
            data.insert(key, value);
        }

        // Add attributes
        data.insert(
            "attributes".to_string(),
            serde_json::json!({
                "class": attributes.get("class").unwrap_or(&String::new()),
                "all": attributes.to_html(),
            }),
        );

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

        // Parse and compile
        let ast = Parser::parse(&template)
            .map_err(|e| RegistryError::RenderError(format!("Parse error: {}", e)))?;

        let mut context = RenderContext::new(Value::Object(data));
        let compiler = Compiler::new();

        compiler
            .compile(&ast, &mut context)
            .map_err(|e| RegistryError::RenderError(format!("Compile error: {}", e)))
    }

    /// Get list of all registered component names
    pub fn component_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .components
            .keys()
            .chain(self.anonymous.keys())
            .cloned()
            .collect();
        names.sort();
        names
    }

    /// Clear all registered components
    pub fn clear(&mut self) {
        self.components.clear();
        self.anonymous.clear();
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::class_component::BaseComponent;
    use super::*;

    #[test]
    fn test_create_registry() {
        let registry = ComponentRegistry::new();
        assert_eq!(registry.component_names().len(), 0);
    }

    #[test]
    fn test_register_component() {
        let mut registry = ComponentRegistry::new();
        let component = BaseComponent::new("alert", "<div>Alert</div>");

        registry.register("alert", component).unwrap();

        assert!(registry.has("alert"));
        assert_eq!(registry.component_names(), vec!["alert"]);
    }

    #[test]
    fn test_register_duplicate_error() {
        let mut registry = ComponentRegistry::new();
        let component1 = BaseComponent::new("alert", "<div>Alert 1</div>");
        let component2 = BaseComponent::new("alert", "<div>Alert 2</div>");

        registry.register("alert", component1).unwrap();
        let result = registry.register("alert", component2);

        assert!(result.is_err());
    }

    #[test]
    fn test_alias() {
        let mut registry = ComponentRegistry::new();
        let component = BaseComponent::new("alert", "<div>Alert</div>");

        registry.register("alert", component).unwrap();
        registry.alias("alert", "notification").unwrap();

        assert!(registry.has("alert"));
        assert!(registry.has("notification"));
    }

    #[test]
    fn test_alias_nonexistent() {
        let mut registry = ComponentRegistry::new();
        let result = registry.alias("nonexistent", "alias");

        assert!(result.is_err());
    }

    #[test]
    fn test_clear() {
        let mut registry = ComponentRegistry::new();
        let component = BaseComponent::new("alert", "<div>Alert</div>");

        registry.register("alert", component).unwrap();
        assert!(registry.has("alert"));

        registry.clear();
        assert!(!registry.has("alert"));
    }
}
