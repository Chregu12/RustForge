//! Email template rendering

use crate::MailError;
use handlebars::Handlebars;
use serde::Serialize;
use std::collections::HashMap;

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
        self.handlebars
            .register_template_string(name, template)?;
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
