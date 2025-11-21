//! # rf-scaffold - Code Scaffolding and Generation for RustForge
//!
//! A powerful scaffolding system for RustForge inspired by Laravel's artisan make commands.
//! Generate models, controllers, migrations, services, and complete project structures with
//! customizable templates.
//!
//! ## Features
//!
//! - **Project Scaffolding**: Create new projects with different templates (API, full-stack, etc.)
//! - **Code Generation**: Generate models, controllers, migrations, services, and more
//! - **Template Engine**: Handlebars-based templates with variable substitution
//! - **Custom Templates**: Register your own templates for project-specific needs
//! - **Smart Naming**: Automatic pluralization, snake_case, PascalCase conversions
//!
//! ## Quick Start
//!
//! ```rust
//! use rf_scaffold::{ScaffoldEngine, ModelOptions};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let scaffold = ScaffoldEngine::new("./my-project")?;
//!
//! // Generate a model
//! scaffold.generate_model("User", &ModelOptions {
//!     fields: vec![
//!         ("name", "String"),
//!         ("email", "String"),
//!         ("age", "i32"),
//!     ],
//!     with_migration: true,
//!     with_factory: false,
//! }).await?;
//!
//! // Generate a controller
//! scaffold.generate_controller("UserController", false).await?;
//!
//! // Generate a migration
//! scaffold.generate_migration("create_users_table").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Generated Code Example
//!
//! **Model:**
//! ```rust,ignore
//! use serde::{Deserialize, Serialize};
//! use chrono::{DateTime, Utc};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub struct User {
//!     pub id: i64,
//!     pub name: String,
//!     pub email: String,
//!     pub created_at: DateTime<Utc>,
//!     pub updated_at: DateTime<Utc>,
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use handlebars::Handlebars;
use serde::Serialize;
use thiserror::Error;
use tokio::fs;
use tokio::sync::RwLock;

pub mod templates;
pub mod generators;
pub mod naming;
pub mod project;

use templates::BuiltinTemplates;
use naming::NamingConvention;

/// Scaffold engine errors
#[derive(Error, Debug)]
pub enum ScaffoldError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("File already exists: {0}")]
    FileExists(PathBuf),

    #[error("Handlebars error: {0}")]
    HandlebarsError(#[from] handlebars::RenderError),
}

pub type ScaffoldResult<T> = Result<T, ScaffoldError>;

/// Options for model generation
#[derive(Debug, Clone)]
pub struct ModelOptions<'a> {
    /// Fields: (name, type)
    pub fields: Vec<(&'a str, &'a str)>,

    /// Generate migration with model
    pub with_migration: bool,

    /// Generate factory with model
    pub with_factory: bool,
}

/// Options for controller generation
#[derive(Debug, Clone)]
pub struct ControllerOptions {
    /// Generate resource controller (CRUD methods)
    pub resource: bool,

    /// API controller (JSON responses)
    pub api: bool,
}

/// Options for project scaffolding
#[derive(Debug, Clone)]
pub struct ProjectOptions<'a> {
    /// Project name
    pub name: &'a str,

    /// Project type
    pub project_type: ProjectType,

    /// Include authentication
    pub with_auth: bool,

    /// Include database
    pub with_database: bool,
}

/// Project type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    /// API-only project
    Api,

    /// Full-stack web project
    FullStack,

    /// Microservice
    Microservice,

    /// CLI application
    Cli,
}

/// Scaffold engine
///
/// # Example
///
/// ```rust
/// use rf_scaffold::ScaffoldEngine;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let scaffold = ScaffoldEngine::new("./my-project")?;
///
/// // Register custom template
/// scaffold.register_template("custom", "Custom: {{name}}").await?;
/// # Ok(())
/// # }
/// ```
pub struct ScaffoldEngine {
    /// Base output path
    base_path: PathBuf,

    /// Handlebars template engine
    handlebars: Arc<RwLock<Handlebars<'static>>>,

    /// Naming convention helper
    naming: NamingConvention,
}

impl ScaffoldEngine {
    /// Create a new scaffold engine
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::ScaffoldEngine;
    ///
    /// let scaffold = ScaffoldEngine::new("./my-project").unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P) -> ScaffoldResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        // Register built-in templates
        BuiltinTemplates::register(&mut handlebars)?;

        Ok(Self {
            base_path,
            handlebars: Arc::new(RwLock::new(handlebars)),
            naming: NamingConvention::new(),
        })
    }

    /// Register a custom template
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::ScaffoldEngine;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let scaffold = ScaffoldEngine::new("./my-project")?;
    ///
    /// scaffold.register_template(
    ///     "my-template",
    ///     "pub struct {{name}} { /* ... */ }"
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_template(&self, name: &str, template: &str) -> ScaffoldResult<()> {
        let mut hb = self.handlebars.write().await;

        hb.register_template_string(name, template)
            .map_err(|e| ScaffoldError::RenderError(e.to_string()))?;

        Ok(())
    }

    /// Generate a model
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::{ScaffoldEngine, ModelOptions};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let scaffold = ScaffoldEngine::new("./my-project")?;
    ///
    /// scaffold.generate_model("User", &ModelOptions {
    ///     fields: vec![("name", "String"), ("email", "String")],
    ///     with_migration: true,
    ///     with_factory: false,
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_model(&self, name: &str, options: &ModelOptions<'_>) -> ScaffoldResult<PathBuf> {
        generators::ModelGenerator::new(self).generate(name, options).await
    }

    /// Generate a controller
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::ScaffoldEngine;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let scaffold = ScaffoldEngine::new("./my-project")?;
    /// scaffold.generate_controller("UserController", true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_controller(&self, name: &str, resource: bool) -> ScaffoldResult<PathBuf> {
        generators::ControllerGenerator::new(self).generate(name, resource).await
    }

    /// Generate a migration
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::ScaffoldEngine;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let scaffold = ScaffoldEngine::new("./my-project")?;
    /// scaffold.generate_migration("create_users_table").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_migration(&self, name: &str) -> ScaffoldResult<PathBuf> {
        generators::MigrationGenerator::new(self).generate(name).await
    }

    /// Generate a service
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_scaffold::ScaffoldEngine;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let scaffold = ScaffoldEngine::new("./my-project")?;
    /// scaffold.generate_service("UserService").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn generate_service(&self, name: &str) -> ScaffoldResult<PathBuf> {
        generators::ServiceGenerator::new(self).generate(name).await
    }

    /// Render a template with data
    pub(crate) async fn render<T: Serialize>(&self, template_name: &str, data: &T) -> ScaffoldResult<String> {
        let hb = self.handlebars.read().await;

        hb.render(template_name, data)
            .map_err(|e| ScaffoldError::RenderError(e.to_string()))
    }

    /// Write content to file
    pub(crate) async fn write_file(&self, path: &Path, content: &str, overwrite: bool) -> ScaffoldResult<()> {
        if path.exists() && !overwrite {
            return Err(ScaffoldError::FileExists(path.to_path_buf()));
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, content).await?;
        Ok(())
    }

    /// Get base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get naming convention helper
    pub fn naming(&self) -> &NamingConvention {
        &self.naming
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_new_scaffold_engine() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();
        assert_eq!(scaffold.base_path(), dir.path());
    }

    #[tokio::test]
    async fn test_register_custom_template() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

        let result = scaffold.register_template("test", "Hello {{name}}").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_render_template() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

        // Don't use blocking call in async test
        let mut hb = scaffold.handlebars.write().await;
        hb.register_template_string("test", "Hello {{name}}").unwrap();
        drop(hb);

        let mut data = HashMap::new();
        data.insert("name", "World");

        let rendered = scaffold.render("test", &data).await.unwrap();
        assert_eq!(rendered, "Hello World");
    }

    #[tokio::test]
    async fn test_write_file() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

        let file_path = dir.path().join("test.txt");
        scaffold.write_file(&file_path, "test content", false).await.unwrap();

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_write_file_exists_error() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

        let file_path = dir.path().join("test.txt");

        // Write first time
        scaffold.write_file(&file_path, "content 1", false).await.unwrap();

        // Try to write again without overwrite
        let result = scaffold.write_file(&file_path, "content 2", false).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ScaffoldError::FileExists(_)));
    }

    #[tokio::test]
    async fn test_write_file_overwrite() {
        let dir = tempdir().unwrap();
        let scaffold = ScaffoldEngine::new(dir.path()).unwrap();

        let file_path = dir.path().join("test.txt");

        // Write first time
        scaffold.write_file(&file_path, "content 1", false).await.unwrap();

        // Overwrite
        scaffold.write_file(&file_path, "content 2", true).await.unwrap();

        let content = fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "content 2");
    }
}
