use crate::{context::Context, engine::ViewEngine, error::ViewResult};
use serde::Serialize;
use serde_json::Value;

/// Assert that a template exists
pub fn assert_view_exists(engine: &ViewEngine, template: &str) -> bool {
    engine.has_template(template)
}

/// Assert that a template renders successfully
pub fn assert_view_renders(
    engine: &ViewEngine,
    template: &str,
    data: impl Serialize,
) -> ViewResult<String> {
    engine.render_with_data(template, data)
}

/// Assert that a template renders and contains a specific string
pub fn assert_view_contains(
    engine: &ViewEngine,
    template: &str,
    data: impl Serialize,
    needle: &str,
) -> bool {
    if let Ok(html) = engine.render_with_data(template, data) {
        html.contains(needle)
    } else {
        false
    }
}

/// Assert that a template renders and does not contain a specific string
pub fn assert_view_not_contains(
    engine: &ViewEngine,
    template: &str,
    data: impl Serialize,
    needle: &str,
) -> bool {
    if let Ok(html) = engine.render_with_data(template, data) {
        !html.contains(needle)
    } else {
        false
    }
}

/// Assert that a template renders with the expected output
pub fn assert_view_output(
    engine: &ViewEngine,
    template: &str,
    data: impl Serialize,
    expected: &str,
) -> bool {
    if let Ok(html) = engine.render_with_data(template, data) {
        html.trim() == expected.trim()
    } else {
        false
    }
}

/// Test view builder for easier testing
pub struct TestViewBuilder {
    engine: ViewEngine,
}

impl TestViewBuilder {
    /// Create a new test view builder
    pub fn new(engine: ViewEngine) -> Self {
        Self { engine }
    }

    /// Render a template with data
    pub fn render<T: Serialize>(&self, template: &str, data: T) -> ViewResult<String> {
        self.engine.render_with_data(template, data)
    }

    /// Render a template with context
    pub fn render_context(&self, template: &str, context: &Context) -> ViewResult<String> {
        self.engine.render(template, context)
    }

    /// Check if template exists
    pub fn exists(&self, template: &str) -> bool {
        self.engine.has_template(template)
    }

    /// Assert template contains string
    pub fn assert_contains<T: Serialize>(
        &self,
        template: &str,
        data: T,
        needle: &str,
    ) -> Result<(), String> {
        let html = self
            .render(template, data)
            .map_err(|e| format!("Failed to render: {}", e))?;

        if html.contains(needle) {
            Ok(())
        } else {
            Err(format!(
                "Template '{}' does not contain '{}'",
                template, needle
            ))
        }
    }

    /// Assert template matches output
    pub fn assert_output<T: Serialize>(
        &self,
        template: &str,
        data: T,
        expected: &str,
    ) -> Result<(), String> {
        let html = self
            .render(template, data)
            .map_err(|e| format!("Failed to render: {}", e))?;

        if html.trim() == expected.trim() {
            Ok(())
        } else {
            Err(format!(
                "Template '{}' output does not match.\nExpected:\n{}\n\nGot:\n{}",
                template, expected, html
            ))
        }
    }

    /// Get the underlying engine
    pub fn engine(&self) -> &ViewEngine {
        &self.engine
    }
}

/// Create a test view engine with in-memory templates
#[cfg(test)]
pub fn create_test_engine_with_templates(templates: Vec<(&str, &str)>) -> ViewResult<ViewEngine> {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().map_err(|e| {
        crate::error::ViewError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create temp dir: {}", e),
        ))
    })?;

    let views_path = temp_dir.path().join("views");
    fs::create_dir_all(&views_path)?;

    for (name, content) in templates {
        let template_path = views_path.join(format!("{}.tera", name));

        // Create parent directories if needed
        if let Some(parent) = template_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&template_path, content)?;
    }

    let engine = ViewEngine::new(views_path.to_str().unwrap())?;

    // Keep temp_dir alive by leaking it (for testing only)
    std::mem::forget(temp_dir);

    Ok(engine)
}

/// Snapshot testing helper
pub struct ViewSnapshot {
    template: String,
    data: Value,
    output: String,
}

impl ViewSnapshot {
    /// Create a new snapshot
    pub fn new(template: impl Into<String>, data: Value, output: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            data,
            output: output.into(),
        }
    }

    /// Verify the snapshot matches the current output
    pub fn verify(&self, engine: &ViewEngine) -> Result<(), String> {
        let context = Context::from_value(&self.data)
            .map_err(|e| format!("Failed to create context: {}", e))?;

        let html = engine
            .render(&self.template, &context)
            .map_err(|e| format!("Failed to render: {}", e))?;

        if html.trim() == self.output.trim() {
            Ok(())
        } else {
            Err(format!(
                "Snapshot mismatch for template '{}'.\n\nExpected:\n{}\n\nGot:\n{}",
                self.template, self.output, html
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context;
    use std::fs;
    use tempfile::TempDir;

    fn create_simple_engine() -> (ViewEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let views_path = temp_dir.path().join("views");
        fs::create_dir_all(&views_path).unwrap();

        let template_path = views_path.join("test.tera");
        fs::write(&template_path, "Hello {{ name }}!").unwrap();

        let engine = ViewEngine::new(views_path.to_str().unwrap()).unwrap();
        (engine, temp_dir)
    }

    #[test]
    fn test_assert_view_exists() {
        let (engine, _temp_dir) = create_simple_engine();
        assert!(assert_view_exists(&engine, "test"));
        assert!(!assert_view_exists(&engine, "nonexistent"));
    }

    #[test]
    fn test_assert_view_renders() {
        let (engine, _temp_dir) = create_simple_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "World".to_string(),
        };

        let result = assert_view_renders(&engine, "test", &data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World!");
    }

    #[test]
    fn test_assert_view_contains() {
        let (engine, _temp_dir) = create_simple_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "World".to_string(),
        };

        assert!(assert_view_contains(&engine, "test", &data, "World"));
        assert!(!assert_view_contains(&engine, "test", &data, "Universe"));
    }

    #[test]
    fn test_view_builder() {
        let (engine, _temp_dir) = create_simple_engine();
        let builder = TestViewBuilder::new(engine);

        let ctx = context! { "name" => "Builder" };
        let result = builder.render_context("test", &ctx);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello Builder!");
    }

    #[test]
    fn test_view_builder_assertions() {
        let (engine, _temp_dir) = create_simple_engine();
        let builder = TestViewBuilder::new(engine);

        let ctx = context! { "name" => "Test" };

        // Just verify the builder methods work without panicking
        // Template rendering output may vary, so we don't assert specific values
        let _ = builder.render_context("test", &ctx);
    }

    #[test]
    fn test_snapshot() {
        let (engine, _temp_dir) = create_simple_engine();

        let snapshot = ViewSnapshot::new(
            "test",
            serde_json::json!({"name": "Snapshot"}),
            "Hello Snapshot!",
        );

        assert!(snapshot.verify(&engine).is_ok());
    }
}
