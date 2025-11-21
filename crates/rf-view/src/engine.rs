use crate::error::{ViewError, ViewResult};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tera::{Context, Tera, Value};

/// Global Tera engine instance
static VIEW_ENGINE: Lazy<Arc<RwLock<Option<Tera>>>> = Lazy::new(|| Arc::new(RwLock::new(None)));

/// View engine for managing Tera templates
///
/// This is a singleton that manages the global Tera instance.
pub struct ViewEngine;

impl ViewEngine {
    /// Initialize the view engine with a glob pattern
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::ViewEngine;
    ///
    /// ViewEngine::init("templates/**/*").expect("Failed to initialize views");
    /// ```
    pub fn init(glob_pattern: &str) -> ViewResult<()> {
        let tera = Tera::new(glob_pattern)
            .map_err(|e| ViewError::InitError(e.to_string()))?;

        let mut engine = VIEW_ENGINE.write()
            .map_err(|e| ViewError::InitError(format!("Lock error: {}", e)))?;

        *engine = Some(tera);

        tracing::info!("View engine initialized with pattern: {}", glob_pattern);

        Ok(())
    }

    /// Initialize from a directory path
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::ViewEngine;
    ///
    /// ViewEngine::init_from_dir("templates").expect("Failed to initialize views");
    /// ```
    pub fn init_from_dir(dir: &str) -> ViewResult<()> {
        let pattern = format!("{}/**/*", dir);
        Self::init(&pattern)
    }

    /// Get a reference to the Tera engine
    pub fn tera() -> ViewResult<Arc<RwLock<Option<Tera>>>> {
        Ok(VIEW_ENGINE.clone())
    }

    /// Render a template with context
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::ViewEngine;
    /// use tera::Context;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut context = Context::new();
    /// context.insert("name", "John");
    ///
    /// let html = ViewEngine::render("welcome.tera", &context)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn render(template: &str, context: &Context) -> ViewResult<String> {
        let engine = VIEW_ENGINE.read()
            .map_err(|e| ViewError::RenderError(format!("Lock error: {}", e)))?;

        let tera = engine.as_ref()
            .ok_or_else(|| ViewError::InitError("View engine not initialized".to_string()))?;

        tera.render(template, context)
            .map_err(|e| ViewError::RenderError(e.to_string()))
    }

    /// Register a custom filter
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::ViewEngine;
    /// use tera::Value;
    /// use std::collections::HashMap;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// ViewEngine::register_filter("uppercase", |value: &Value, _: &HashMap<String, Value>| {
    ///     Ok(Value::String(value.as_str().unwrap_or("").to_uppercase()))
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_filter<F>(name: &str, filter: F) -> ViewResult<()>
    where
        F: Fn(&Value, &HashMap<String, Value>) -> tera::Result<Value> + Send + Sync + 'static,
    {
        let mut engine = VIEW_ENGINE.write()
            .map_err(|e| ViewError::InitError(format!("Lock error: {}", e)))?;

        let tera = engine.as_mut()
            .ok_or_else(|| ViewError::InitError("View engine not initialized".to_string()))?;

        tera.register_filter(name, filter);

        tracing::debug!("Registered custom filter: {}", name);

        Ok(())
    }

    /// Register a custom function
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::ViewEngine;
    /// use tera::Value;
    /// use std::collections::HashMap;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// ViewEngine::register_function("now", |_: &HashMap<String, Value>| {
    ///     Ok(Value::String(chrono::Utc::now().to_rfc3339()))
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_function<F>(name: &str, function: F) -> ViewResult<()>
    where
        F: Fn(&HashMap<String, Value>) -> tera::Result<Value> + Send + Sync + 'static,
    {
        let mut engine = VIEW_ENGINE.write()
            .map_err(|e| ViewError::InitError(format!("Lock error: {}", e)))?;

        let tera = engine.as_mut()
            .ok_or_else(|| ViewError::InitError("View engine not initialized".to_string()))?;

        tera.register_function(name, function);

        tracing::debug!("Registered custom function: {}", name);

        Ok(())
    }

    /// Reload all templates (useful in development)
    pub fn reload() -> ViewResult<()> {
        let mut engine = VIEW_ENGINE.write()
            .map_err(|e| ViewError::InitError(format!("Lock error: {}", e)))?;

        if let Some(tera) = engine.as_mut() {
            tera.full_reload()
                .map_err(|e| ViewError::InitError(format!("Reload failed: {}", e)))?;

            tracing::info!("Templates reloaded");
        }

        Ok(())
    }

    /// Check if a template exists
    pub fn has_template(name: &str) -> ViewResult<bool> {
        let engine = VIEW_ENGINE.read()
            .map_err(|e| ViewError::RenderError(format!("Lock error: {}", e)))?;

        let tera = engine.as_ref()
            .ok_or_else(|| ViewError::InitError("View engine not initialized".to_string()))?;

        let result = tera.get_template_names().any(|t| t == name);
        Ok(result)
    }

    /// Get all registered template names
    pub fn template_names() -> ViewResult<Vec<String>> {
        let engine = VIEW_ENGINE.read()
            .map_err(|e| ViewError::RenderError(format!("Lock error: {}", e)))?;

        let tera = engine.as_ref()
            .ok_or_else(|| ViewError::InitError("View engine not initialized".to_string()))?;

        let names: Vec<String> = tera.get_template_names().map(|s| s.to_string()).collect();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_engine_api() {
        // This test just checks that the API compiles
        // Real tests would need actual template files
    }
}
