//! CLI Code Generation for RustForge
//!
//! This crate provides code scaffolding and generation tools.

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

/// Generation errors
#[derive(Debug, Error)]
pub enum GeneratorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("File already exists: {0}")]
    FileExists(PathBuf),

    #[error("Invalid name: {0}")]
    InvalidName(String),
}

pub type GeneratorResult<T> = Result<T, GeneratorError>;

/// Generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Name of the item to generate
    pub name: String,
    /// Output directory
    pub output_dir: PathBuf,
    /// Additional template data
    pub data: serde_json::Value,
    /// Overwrite existing files
    pub force: bool,
}

impl GeneratorConfig {
    /// Create a new generator config
    pub fn new(name: impl Into<String>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            output_dir: output_dir.into(),
            data: serde_json::json!({}),
            force: false,
        }
    }

    /// Add custom data
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Enable file overwriting
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Template data for generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateData {
    /// Name of the item
    pub name: String,
    /// Snake case name
    pub snake_name: String,
    /// Pascal case name
    pub pascal_name: String,
    /// Timestamp
    pub timestamp: String,
    /// Custom data
    #[serde(flatten)]
    pub custom: serde_json::Value,
}

impl TemplateData {
    /// Create from config
    pub fn from_config(config: &GeneratorConfig) -> Self {
        let name = config.name.clone();
        let snake_name = to_snake_case(&name);
        let pascal_name = to_pascal_case(&name);

        Self {
            name,
            snake_name,
            pascal_name,
            timestamp: chrono::Utc::now().to_rfc3339(),
            custom: config.data.clone(),
        }
    }
}

/// Model generator
pub struct ModelGenerator {
    handlebars: Handlebars<'static>,
}

impl ModelGenerator {
    /// Create a new model generator
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        // Register model template
        handlebars
            .register_template_string(
                "model",
                r#"
//! {{pascal_name}} model
//! Generated at {{timestamp}}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{pascal_name}} {
    pub id: i64,
    // Add your fields here
}

impl {{pascal_name}} {
    /// Create a new {{name}}
    pub fn new() -> Self {
        Self {
            id: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_{{snake_name}}_creation() {
        let {{snake_name}} = {{pascal_name}}::new();
        assert_eq!({{snake_name}}.id, 0);
    }
}
"#,
            )
            .unwrap();

        Self { handlebars }
    }

    /// Generate a model file
    pub async fn generate(&self, config: GeneratorConfig) -> GeneratorResult<PathBuf> {
        let data = TemplateData::from_config(&config);
        let content = self
            .handlebars
            .render("model", &data)
            .map_err(|e| GeneratorError::Template(e.to_string()))?;

        let file_path = config
            .output_dir
            .join(format!("{}.rs", data.snake_name));

        write_file(&file_path, &content, config.force).await?;
        Ok(file_path)
    }
}

impl Default for ModelGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Controller generator
pub struct ControllerGenerator {
    handlebars: Handlebars<'static>,
}

impl ControllerGenerator {
    /// Create a new controller generator
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_template_string(
                "controller",
                r#"
//! {{pascal_name}} controller
//! Generated at {{timestamp}}

use axum::{
    routing::{get, post, put, delete},
    Router, Json, extract::Path,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct {{pascal_name}}Response {
    pub id: i64,
    // Add your response fields
}

pub fn {{snake_name}}_routes() -> Router {
    Router::new()
        .route("/{{name}}", get(index).post(store))
        .route("/{{name}}/:id", get(show).put(update).delete(destroy))
}

/// List all {{name}}s
async fn index() -> Json<Vec<{{pascal_name}}Response>> {
    // TODO: Implement
    Json(vec![])
}

/// Create a new {{name}}
async fn store() -> Json<{{pascal_name}}Response> {
    // TODO: Implement
    Json({{pascal_name}}Response { id: 1 })
}

/// Show a single {{name}}
async fn show(Path(id): Path<i64>) -> Json<{{pascal_name}}Response> {
    // TODO: Implement
    Json({{pascal_name}}Response { id })
}

/// Update a {{name}}
async fn update(Path(id): Path<i64>) -> Json<{{pascal_name}}Response> {
    // TODO: Implement
    Json({{pascal_name}}Response { id })
}

/// Delete a {{name}}
async fn destroy(Path(id): Path<i64>) -> Json<()> {
    // TODO: Implement
    Json(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_{{snake_name}}_routes() {
        let _router = {{snake_name}}_routes();
    }
}
"#,
            )
            .unwrap();

        Self { handlebars }
    }

    /// Generate a controller file
    pub async fn generate(&self, config: GeneratorConfig) -> GeneratorResult<PathBuf> {
        let data = TemplateData::from_config(&config);
        let content = self
            .handlebars
            .render("controller", &data)
            .map_err(|e| GeneratorError::Template(e.to_string()))?;

        let file_path = config
            .output_dir
            .join(format!("{}_controller.rs", data.snake_name));

        write_file(&file_path, &content, config.force).await?;
        Ok(file_path)
    }
}

impl Default for ControllerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Test generator
pub struct TestGenerator {
    handlebars: Handlebars<'static>,
}

impl TestGenerator {
    /// Create a new test generator
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();

        handlebars
            .register_template_string(
                "test",
                r#"
//! Tests for {{pascal_name}}
//! Generated at {{timestamp}}

#[cfg(test)]
mod {{snake_name}}_tests {
    use super::*;

    #[test]
    fn test_{{snake_name}}_example() {
        // TODO: Implement test
        assert!(true);
    }

    #[tokio::test]
    async fn test_{{snake_name}}_async() {
        // TODO: Implement async test
        assert!(true);
    }
}
"#,
            )
            .unwrap();

        Self { handlebars }
    }

    /// Generate a test file
    pub async fn generate(&self, config: GeneratorConfig) -> GeneratorResult<PathBuf> {
        let data = TemplateData::from_config(&config);
        let content = self
            .handlebars
            .render("test", &data)
            .map_err(|e| GeneratorError::Template(e.to_string()))?;

        let file_path = config
            .output_dir
            .join(format!("{}_test.rs", data.snake_name));

        write_file(&file_path, &content, config.force).await?;
        Ok(file_path)
    }
}

impl Default for TestGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_upper = true;
        } else {
            result.push(c);
            prev_upper = false;
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    s.split(&['_', '-'][..])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect()
}

async fn write_file(path: &Path, content: &str, force: bool) -> GeneratorResult<()> {
    // Check if file exists
    if !force && path.exists() {
        return Err(GeneratorError::FileExists(path.to_path_buf()));
    }

    // Create parent directories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    // Write file
    fs::write(path, content).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("UserModel"), "user_model");
        assert_eq!(to_snake_case("PostController"), "post_controller");
        assert_eq!(to_snake_case("HTTPRequest"), "h_t_t_p_request");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("user_model"), "UserModel");
        assert_eq!(to_pascal_case("post-controller"), "PostController");
        assert_eq!(to_pascal_case("my_test_name"), "MyTestName");
    }

    #[test]
    fn test_generator_config() {
        let config = GeneratorConfig::new("User", "src/models")
            .with_data(serde_json::json!({"extra": "data"}))
            .force();

        assert_eq!(config.name, "User");
        assert!(config.force);
    }

    #[tokio::test]
    async fn test_model_generator() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = GeneratorConfig::new("User", temp_dir.path());

        let generator = ModelGenerator::new();
        let path = generator.generate(config).await.unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("pub struct User"));
    }

    #[tokio::test]
    async fn test_controller_generator() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = GeneratorConfig::new("Post", temp_dir.path());

        let generator = ControllerGenerator::new();
        let path = generator.generate(config).await.unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("post_routes"));
    }

    #[tokio::test]
    async fn test_test_generator() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = GeneratorConfig::new("Article", temp_dir.path());

        let generator = TestGenerator::new();
        let path = generator.generate(config).await.unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("article_tests"));
    }

    #[tokio::test]
    async fn test_file_overwrite_protection() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = GeneratorConfig::new("Test", temp_dir.path());

        let generator = ModelGenerator::new();
        generator.generate(config.clone()).await.unwrap();

        // Should fail without force
        let result = generator.generate(config.clone()).await;
        assert!(result.is_err());

        // Should succeed with force
        let config_force = config.force();
        let result = generator.generate(config_force).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_template_data() {
        let config = GeneratorConfig::new("UserAccount", "src");
        let data = TemplateData::from_config(&config);

        assert_eq!(data.name, "UserAccount");
        assert_eq!(data.snake_name, "user_account");
        assert_eq!(data.pascal_name, "UserAccount");
        assert!(!data.timestamp.is_empty());
    }
}
