//! Email template rendering

use crate::MailError;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;

/// Process-global shared mail [`TemplateEngine`].
///
/// This is the registry that [`MailBuilder::view`](crate::MailBuilder::view)
/// renders against when no per-builder engine was set with
/// [`MailBuilder::with_template_engine`](crate::MailBuilder::with_template_engine).
/// Register your email templates once at boot (via [`register_template`]) and
/// every `MailBuilder::new().view("name", data)` call resolves them — instead of
/// each builder spinning up a fresh empty engine that can only ever report
/// "Template not found".
pub static GLOBAL_TEMPLATE_ENGINE: Lazy<RwLock<TemplateEngine>> =
    Lazy::new(|| RwLock::new(TemplateEngine::new()));

/// Register a named template string into the process-global shared engine
/// ([`GLOBAL_TEMPLATE_ENGINE`]) so [`MailBuilder::view`](crate::MailBuilder::view)
/// can render it.
///
/// # Example
///
/// ```
/// use rf_mail::{templates, MailBuilder};
/// use serde_json::json;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// templates::register_template("welcome", "<h1>Hello, {{name}}!</h1>")?;
///
/// let mail = MailBuilder::new()
///     .from(rf_mail::Address::new("noreply@example.com"))
///     .to(rf_mail::Address::new("user@example.com"))
///     .subject("Welcome")
///     .view("welcome", json!({ "name": "Alice" }))?
///     .build()?;
///
/// assert_eq!(mail.html(), Some("<h1>Hello, Alice!</h1>"));
/// # Ok(())
/// # }
/// ```
pub fn register_template(name: &str, template: &str) -> Result<(), MailError> {
    GLOBAL_TEMPLATE_ENGINE
        .write()
        .map_err(|e| MailError::ConfigError(format!("template engine lock poisoned: {e}")))?
        .register_template(name, template)
}

/// Render a template that was registered into the process-global shared engine
/// ([`GLOBAL_TEMPLATE_ENGINE`]).
///
/// Used by [`MailBuilder::view`](crate::MailBuilder::view) to resolve named
/// templates against the shared registry.
pub fn render_global<T: Serialize>(name: &str, data: &T) -> Result<String, MailError> {
    GLOBAL_TEMPLATE_ENGINE
        .read()
        .map_err(|e| MailError::ConfigError(format!("template engine lock poisoned: {e}")))?
        .render(name, data)
}

/// Template engine for email rendering
///
/// # Example
///
/// ```
/// use rf_mail::TemplateEngine;
/// use serde_json::json;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut engine = TemplateEngine::new();
///
/// engine.register_template("welcome", "Hello, {{name}}!")?;
///
/// let data = json!({"name": "Alice"});
/// let rendered = engine.render("welcome", &data)?;
///
/// assert_eq!(rendered, "Hello, Alice!");
/// # Ok(())
/// # }
/// ```
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    /// Create new template engine
    pub fn new() -> Self {
        Self {
            handlebars: Handlebars::new(),
        }
    }

    /// Register a template by name
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::TemplateEngine;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut engine = TemplateEngine::new();
    /// engine.register_template("greeting", "Hello, {{name}}!")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_template(&mut self, name: &str, template: &str) -> Result<(), MailError> {
        self.handlebars.register_template_string(name, template)?;
        Ok(())
    }

    /// Register multiple templates at once
    pub fn register_templates(
        &mut self,
        templates: HashMap<String, String>,
    ) -> Result<(), MailError> {
        for (name, template) in templates {
            self.register_template(&name, &template)?;
        }
        Ok(())
    }

    /// Render a template with data
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::TemplateEngine;
    /// use serde_json::json;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut engine = TemplateEngine::new();
    /// engine.register_template("welcome", "Hello, {{name}}!")?;
    ///
    /// let data = json!({"name": "Bob"});
    /// let result = engine.render("welcome", &data)?;
    /// assert_eq!(result, "Hello, Bob!");
    /// # Ok(())
    /// # }
    /// ```
    pub fn render<T: Serialize>(&self, name: &str, data: &T) -> Result<String, MailError> {
        Ok(self.handlebars.render(name, data)?)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_template_engine() {
        let mut engine = TemplateEngine::new();

        engine
            .register_template("test", "Hello, {{name}}!")
            .unwrap();

        let data = json!({"name": "World"});
        let result = engine.render("test", &data).unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_template_engine_complex() {
        let mut engine = TemplateEngine::new();

        engine
            .register_template(
                "email",
                r#"
                <h1>Hello, {{user.name}}!</h1>
                <p>Your email is {{user.email}}</p>
                "#,
            )
            .unwrap();

        let data = json!({
            "user": {
                "name": "Alice",
                "email": "alice@example.com"
            }
        });

        let result = engine.render("email", &data).unwrap();
        assert!(result.contains("Alice"));
        assert!(result.contains("alice@example.com"));
    }
}
