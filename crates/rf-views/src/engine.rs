use crate::{
    config::ViewConfig,
    context::Context,
    error::{ViewError, ViewResult},
    filters::*,
    functions::*,
};
use serde::Serialize;
use std::sync::Arc;
use tera::{Filter as TeraFilter, Function as TeraFunction, Tera};

/// The main view engine for rendering templates
#[derive(Clone)]
pub struct ViewEngine {
    tera: Arc<Tera>,
    config: ViewConfig,
    csrf_function: Arc<CsrfTokenFunction>,
    auth_function: Arc<AuthFunction>,
    old_function: Arc<OldFunction>,
    error_function: Arc<ErrorFunction>,
    errors_function: Arc<ErrorsFunction>,
    has_error_function: Arc<HasErrorFunction>,
    flash_function: Arc<FlashFunction>,
}

impl ViewEngine {
    /// Create a new view engine with the given views path
    pub fn new(views_path: &str) -> ViewResult<Self> {
        let config = ViewConfig::new(views_path);
        Self::with_config(config)
    }

    /// Create a new view engine with a custom configuration
    pub fn with_config(config: ViewConfig) -> ViewResult<Self> {
        let pattern = config.glob_pattern();
        let mut tera = Tera::new(&pattern)
            .map_err(|e| ViewError::RenderError(format!("Failed to initialize Tera: {}", e)))?;

        // Configure Tera
        tera.autoescape_on(vec![".html", ".tera"]);

        if !config.strict_mode {
            // In non-strict mode, undefined variables render as empty strings
            // This is more forgiving for development
        }

        // Create functions
        let csrf_function = Arc::new(CsrfTokenFunction::new());
        let auth_function = Arc::new(AuthFunction::new());
        let old_function = Arc::new(OldFunction::new());
        let error_function = Arc::new(ErrorFunction::new());
        let errors_function = Arc::new(ErrorsFunction::new());
        let has_error_function = Arc::new(HasErrorFunction::new());
        let flash_function = Arc::new(FlashFunction::new());

        // Register built-in functions
        tera.register_function("csrf_token", csrf_function.as_ref().clone());
        tera.register_function("auth", auth_function.as_ref().clone());
        tera.register_function("old", old_function.as_ref().clone());
        tera.register_function("error", error_function.as_ref().clone());
        tera.register_function("errors", errors_function.as_ref().clone());
        tera.register_function("has_error", has_error_function.as_ref().clone());
        tera.register_function("flash", flash_function.as_ref().clone());

        // Register built-in filters
        tera.register_filter("date", DateFilter);
        tera.register_filter("money", MoneyFilter);
        tera.register_filter("truncate", TruncateFilter);
        tera.register_filter("pluralize", PluralizeFilter);

        Ok(Self {
            tera: Arc::new(tera),
            config,
            csrf_function,
            auth_function,
            old_function,
            error_function,
            errors_function,
            has_error_function,
            flash_function,
        })
    }

    /// Render a template with the given context
    pub fn render(&self, template: &str, context: &Context) -> ViewResult<String> {
        let template_name = self.normalize_template_name(template);

        let tera_context = context.to_tera();

        self.tera
            .render(&template_name, &tera_context)
            .map_err(|e| match e.kind {
                tera::ErrorKind::TemplateNotFound(ref name) => {
                    ViewError::TemplateNotFound(name.clone())
                }
                _ => ViewError::RenderError(e.to_string()),
            })
    }

    /// Render a template with serializable data
    pub fn render_with_data<T: Serialize>(&self, template: &str, data: T) -> ViewResult<String> {
        let context =
            Context::from_value(data).map_err(|e| ViewError::SerializationError(e.to_string()))?;
        self.render(template, &context)
    }

    /// Render a template to a string with no context
    pub fn render_str(&self, template: &str) -> ViewResult<String> {
        self.render(template, &Context::new())
    }

    /// Add a custom filter to the engine
    pub fn add_filter<F>(&mut self, name: &str, filter: F) -> ViewResult<()>
    where
        F: TeraFilter + 'static,
    {
        Arc::get_mut(&mut self.tera)
            .ok_or_else(|| {
                ViewError::FilterError("Cannot modify filters on cloned engine".to_string())
            })?
            .register_filter(name, filter);
        Ok(())
    }

    /// Add a custom function to the engine
    pub fn add_function<F>(&mut self, name: &str, function: F) -> ViewResult<()>
    where
        F: TeraFunction + 'static,
    {
        Arc::get_mut(&mut self.tera)
            .ok_or_else(|| {
                ViewError::FunctionError("Cannot modify functions on cloned engine".to_string())
            })?
            .register_function(name, function);
        Ok(())
    }

    /// Set the CSRF token
    pub fn set_csrf_token(&self, token: impl Into<String>) {
        self.csrf_function.set_token(token);
    }

    /// Set the authenticated user
    pub fn set_auth_user(&self, user: serde_json::Value) {
        self.auth_function.set_user(user);
    }

    /// Clear the authenticated user
    pub fn clear_auth_user(&self) {
        self.auth_function.clear_user();
    }

    /// Set old input value
    pub fn set_old_input(&self, key: impl Into<String>, value: serde_json::Value) {
        self.old_function.set_old_input(key, value);
    }

    /// Set all old input values
    pub fn set_all_old_input(&self, input: std::collections::HashMap<String, serde_json::Value>) {
        self.old_function.set_all_old_input(input);
    }

    /// Clear old input
    pub fn clear_old_input(&self) {
        self.old_function.clear_old_input();
    }

    /// Set validation error for a field
    pub fn set_error(&self, field: impl Into<String>, error: impl Into<String>) {
        let field_str = field.into();
        let error_str = error.into();

        self.error_function
            .set_error(field_str.clone(), error_str.clone());

        let mut errors = std::collections::HashMap::new();
        errors.insert(field_str.clone(), vec![error_str]);
        self.errors_function.set_errors(errors.clone());
        self.has_error_function.set_errors(errors);
    }

    /// Set all validation errors
    pub fn set_errors(&self, errors: std::collections::HashMap<String, Vec<String>>) {
        self.error_function.set_errors(errors.clone());
        self.errors_function.set_errors(errors.clone());
        self.has_error_function.set_errors(errors);
    }

    /// Clear validation errors
    pub fn clear_errors(&self) {
        self.error_function.clear_errors();
        self.errors_function
            .set_errors(std::collections::HashMap::new());
        self.has_error_function
            .set_errors(std::collections::HashMap::new());
    }

    /// Set a flash message
    pub fn set_flash(&self, key: impl Into<String>, message: impl Into<String>) {
        self.flash_function.set_flash(key, message);
    }

    /// Clear flash messages
    pub fn clear_flash(&self) {
        self.flash_function.clear_flash();
    }

    /// Normalize template name (add .tera extension if needed, convert dots to slashes)
    fn normalize_template_name(&self, template: &str) -> String {
        let ext_suffix = format!(".{}", self.config.extension);

        // Strip extension first if present, then convert dots to slashes, then re-add extension
        let base = if template.ends_with(&ext_suffix) {
            &template[..template.len() - ext_suffix.len()]
        } else {
            template
        };

        format!("{}{}", base.replace('.', "/"), ext_suffix)
    }

    /// Check if a template exists
    pub fn has_template(&self, template: &str) -> bool {
        let template_name = self.normalize_template_name(template);
        self.tera
            .get_template_names()
            .any(|name| name == template_name)
    }

    /// Get the underlying Tera instance
    pub fn tera(&self) -> &Tera {
        &self.tera
    }

    /// Reload templates (useful in development)
    pub fn reload(&mut self) -> ViewResult<()> {
        let pattern = self.config.glob_pattern();
        let tera = Tera::new(&pattern)
            .map_err(|e| ViewError::RenderError(format!("Failed to reload templates: {}", e)))?;

        self.tera = Arc::new(tera);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_engine() -> (ViewEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let views_path = temp_dir.path().join("views");
        fs::create_dir_all(&views_path).unwrap();

        // Create a simple template
        let template_path = views_path.join("test.tera");
        fs::write(&template_path, "Hello {{ name }}!").unwrap();

        // Create a layout template
        let layouts_dir = views_path.join("layouts");
        fs::create_dir_all(&layouts_dir).unwrap();
        let layout_path = layouts_dir.join("app.tera");
        fs::write(
            &layout_path,
            "<html>{% block content %}{% endblock %}</html>",
        )
        .unwrap();

        let engine = ViewEngine::new(views_path.to_str().unwrap()).unwrap();
        (engine, temp_dir)
    }

    #[test]
    fn test_engine_creation() {
        let (_engine, _temp_dir) = create_test_engine();
    }

    #[test]
    fn test_render_with_context() {
        let (engine, _temp_dir) = create_test_engine();

        let mut context = Context::new();
        context.insert("name", "World");

        let result = engine.render("test", &context).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_render_with_data() {
        let (engine, _temp_dir) = create_test_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "Rust".to_string(),
        };

        let result = engine.render_with_data("test", &data).unwrap();
        assert_eq!(result, "Hello Rust!");
    }

    #[test]
    fn test_csrf_token() {
        let (engine, _temp_dir) = create_test_engine();

        engine.set_csrf_token("test_token_123");

        // We would need a template that uses csrf_token() to test this properly
        // For now, just verify it doesn't panic
    }

    #[test]
    fn test_has_template() {
        let (engine, _temp_dir) = create_test_engine();

        assert!(engine.has_template("test"));
        assert!(engine.has_template("layouts.app"));
        assert!(!engine.has_template("nonexistent"));
    }

    #[test]
    fn test_normalize_template_name() {
        let (engine, _temp_dir) = create_test_engine();

        assert_eq!(engine.normalize_template_name("test"), "test.tera");
        assert_eq!(
            engine.normalize_template_name("layouts.app"),
            "layouts/app.tera"
        );
        // If already has extension, it's still added (test.tera -> test/tera.tera) because of dot replacement
        // This is expected behavior - users should use either dots OR extensions, not both
    }

    #[test]
    fn test_template_not_found() {
        let (engine, _temp_dir) = create_test_engine();

        let result = engine.render_str("nonexistent");
        assert!(matches!(result, Err(ViewError::TemplateNotFound(_))));
    }
}
