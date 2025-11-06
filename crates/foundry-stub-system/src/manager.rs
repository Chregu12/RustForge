use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;
use tracing::{debug, warn};
use tera::{Tera, Context};

use crate::error::{Result, StubError};
use crate::stub::{Stub, StubType, DefaultStubs};
use crate::context::StubContext;
use crate::variables::StubVariables;

/// Manager for loading and rendering stubs
pub struct StubManager {
    custom_stubs_dir: PathBuf,
    tera: Tera,
    custom_stubs: HashMap<String, Stub>,
}

impl StubManager {
    /// Create a new stub manager
    pub fn new(custom_stubs_dir: impl AsRef<Path>) -> Self {
        Self {
            custom_stubs_dir: custom_stubs_dir.as_ref().to_path_buf(),
            tera: Tera::default(),
            custom_stubs: HashMap::new(),
        }
    }

    /// Load all custom stubs from the directory
    pub async fn load_custom_stubs(&mut self) -> Result<()> {
        let stubs_dir = &self.custom_stubs_dir;

        if !stubs_dir.exists() {
            debug!("Custom stubs directory does not exist: {:?}", stubs_dir);
            return Ok(());
        }

        let mut entries = fs::read_dir(stubs_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("stub") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| StubError::InvalidFormat("Invalid stub filename".to_string()))?;

                let content = fs::read_to_string(&path).await?;

                let stub_type = StubType::from_str(name).unwrap_or(StubType::Custom);

                let stub = Stub::new(name, stub_type, content).custom();

                debug!("Loaded custom stub: {}", name);
                self.custom_stubs.insert(name.to_string(), stub);
            }
        }

        Ok(())
    }

    /// Get a stub by name (custom first, then default)
    pub fn get_stub(&self, name: &str) -> Result<Stub> {
        // Check custom stubs first
        if let Some(stub) = self.custom_stubs.get(name) {
            debug!("Using custom stub: {}", name);
            return Ok(stub.clone());
        }

        // Fall back to default stubs
        let stub_type = StubType::from_str(name)
            .ok_or_else(|| StubError::StubNotFound(name.to_string()))?;

        let content = match stub_type {
            StubType::Model => DefaultStubs::model(),
            StubType::Controller => DefaultStubs::controller(),
            StubType::Service => DefaultStubs::service(),
            StubType::Migration => DefaultStubs::migration(),
            StubType::Test => DefaultStubs::test(),
            _ => return Err(StubError::StubNotFound(name.to_string())),
        };

        debug!("Using default stub: {}", name);
        Ok(Stub::new(name, stub_type, content))
    }

    /// Render a stub with the given context
    pub async fn render(&self, stub_name: &str, context: StubContext) -> Result<String> {
        let stub = self.get_stub(stub_name)?;

        // Create variables from context
        let mut variables = StubVariables::new(&context.name)
            .with_namespace(&context.namespace);

        // Add custom variables from context
        for (key, value) in &context.custom {
            variables = variables.with_custom(key, value);
        }

        // Build properties string if needed
        let properties = self.build_properties(&context.properties);
        variables = variables.with_custom("properties", properties);

        // Create Tera context
        let var_context = variables.to_context();
        let mut tera_context = Context::new();

        for (key, value) in var_context {
            tera_context.insert(&key, &value);
        }

        // Add special pluralized variables
        let snake_plural = format!("{}",
            crate::variables::CaseConverter::plural(&variables.snake));
        tera_context.insert("snake_plural", &snake_plural);

        let studly_plural = format!("{}",
            crate::variables::CaseConverter::plural(&variables.studly));
        tera_context.insert("studly_plural", &studly_plural);

        // Render the template
        let rendered = Tera::one_off(&stub.content, &tera_context, false)?;

        Ok(rendered)
    }

    /// Build properties string for struct fields
    fn build_properties(&self, properties: &HashMap<String, String>) -> String {
        if properties.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        for (name, type_) in properties {
            result.push_str(&format!("    pub {}: {},\n", name, type_));
        }
        result
    }

    /// List all available stubs
    pub fn list_stubs(&self) -> Vec<String> {
        let mut stubs: Vec<String> = Vec::new();

        // Add custom stubs
        for name in self.custom_stubs.keys() {
            stubs.push(format!("{} (custom)", name));
        }

        // Add default stubs
        let defaults = vec!["model", "controller", "service", "migration", "test"];
        for name in defaults {
            if !self.custom_stubs.contains_key(name) {
                stubs.push(format!("{} (default)", name));
            }
        }

        stubs.sort();
        stubs
    }

    /// Check if a custom stub exists
    pub fn has_custom_stub(&self, name: &str) -> bool {
        self.custom_stubs.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = StubManager::new("./stubs");
        assert_eq!(manager.custom_stubs.len(), 0);
    }

    #[tokio::test]
    async fn test_get_default_stub() {
        let manager = StubManager::new("./stubs");
        let stub = manager.get_stub("model");
        assert!(stub.is_ok());
        assert_eq!(stub.unwrap().stub_type, StubType::Model);
    }

    #[tokio::test]
    async fn test_render_model_stub() {
        let manager = StubManager::new("./stubs");

        let context = StubContext::new("User")
            .with_namespace("app::models")
            .with_property("name", "String")
            .with_property("email", "String");

        let result = manager.render("model", context).await;
        assert!(result.is_ok());

        let rendered = result.unwrap();
        assert!(rendered.contains("pub struct Model"));
        assert!(rendered.contains("pub name: String"));
    }

    #[tokio::test]
    async fn test_load_custom_stubs() {
        let temp_dir = TempDir::new().unwrap();
        let stubs_dir = temp_dir.path();

        // Create a custom stub file
        let custom_stub_path = stubs_dir.join("model.stub");
        fs::write(&custom_stub_path, "Custom Model: {{ studly }}")
            .await
            .unwrap();

        let mut manager = StubManager::new(stubs_dir);
        manager.load_custom_stubs().await.unwrap();

        assert!(manager.has_custom_stub("model"));
        assert_eq!(manager.custom_stubs.len(), 1);
    }

    #[tokio::test]
    async fn test_custom_stub_priority() {
        let temp_dir = TempDir::new().unwrap();
        let stubs_dir = temp_dir.path();

        // Create a custom model stub
        let custom_stub = "CUSTOM: {{ studly }}";
        let custom_stub_path = stubs_dir.join("model.stub");
        fs::write(&custom_stub_path, custom_stub).await.unwrap();

        let mut manager = StubManager::new(stubs_dir);
        manager.load_custom_stubs().await.unwrap();

        let stub = manager.get_stub("model").unwrap();
        assert!(stub.is_custom);
        assert!(stub.content.contains("CUSTOM"));
    }

    #[tokio::test]
    async fn test_list_stubs() {
        let manager = StubManager::new("./stubs");
        let stubs = manager.list_stubs();
        assert!(!stubs.is_empty());
        assert!(stubs.iter().any(|s| s.contains("model")));
    }

    #[tokio::test]
    async fn test_render_with_variables() {
        let manager = StubManager::new("./stubs");

        let context = StubContext::new("BlogPost")
            .with_namespace("app::models");

        let result = manager.render("controller", context).await;
        assert!(result.is_ok());

        let rendered = result.unwrap();
        assert!(rendered.contains("BlogPostController"));
        assert!(rendered.contains("blog_post"));
    }
}
