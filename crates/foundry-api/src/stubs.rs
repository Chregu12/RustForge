/// Stub Management System for Code Generation
///
/// Provides a flexible system for managing code generation stubs/templates,
/// allowing users to customize templates for `make:*` commands.
///
/// # Example
///
/// ```rust,no_run
/// use foundry_api::stubs::{StubManager, StubVariables};
///
/// let manager = StubManager::new("stubs");
///
/// // Get a stub
/// let stub = manager.get("model")?;
///
/// // Render with variables
/// let mut vars = StubVariables::new();
/// vars.set("namespace", "App\\Models");
/// vars.set("name", "User");
/// vars.set("table", "users");
///
/// let rendered = stub.render(&vars)?;
/// println!("{}", rendered);
/// ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// A code generation stub/template
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stub {
    /// Stub identifier (e.g., "model", "controller", "migration")
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Template content
    pub content: String,
    /// Output file extension
    pub extension: String,
    /// Whether this is a built-in stub
    pub builtin: bool,
    /// Path to stub file (if loaded from filesystem)
    pub path: Option<PathBuf>,
}

impl Stub {
    /// Create a new stub
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
        extension: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            content: content.into(),
            extension: extension.into(),
            builtin: false,
            path: None,
        }
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as built-in stub
    pub fn builtin(mut self) -> Self {
        self.builtin = true;
        self
    }

    /// Set the file path
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Render the stub with variables
    pub fn render(&self, variables: &StubVariables) -> Result<String, StubError> {
        let mut content = self.content.clone();

        // Replace all variables
        for (key, value) in variables.variables() {
            let placeholder = format!("{{{{{}}}}}", key);
            content = content.replace(&placeholder, &value);
        }

        Ok(content)
    }

    /// Get all variable placeholders in the stub
    pub fn get_placeholders(&self) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut in_placeholder = false;
        let mut current = String::new();

        for ch in self.content.chars() {
            match ch {
                '{' if !in_placeholder => {
                    in_placeholder = true;
                }
                '}' if in_placeholder => {
                    if !current.is_empty() {
                        placeholders.push(current.clone());
                        current.clear();
                    }
                    in_placeholder = false;
                }
                _ if in_placeholder => {
                    current.push(ch);
                }
                _ => {}
            }
        }

        placeholders
    }
}

/// Variables for stub rendering
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StubVariables {
    variables: HashMap<String, String>,
}

impl StubVariables {
    /// Create new stub variables
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Set a variable
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Get a variable
    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }

    /// Check if variable exists
    pub fn has(&self, key: &str) -> bool {
        self.variables.contains_key(key)
    }

    /// Get all variables
    pub fn variables(&self) -> &HashMap<String, String> {
        &self.variables
    }

    /// Merge with another set of variables
    pub fn merge(mut self, other: &StubVariables) -> Self {
        for (key, value) in &other.variables {
            self.variables.insert(key.clone(), value.clone());
        }
        self
    }

    /// Add common variables based on command name
    pub fn with_common_vars(mut self, command: &str) -> Self {
        self.set("command", command);
        self.set("timestamp", chrono::Utc::now().to_rfc3339());
        self.set("year", chrono::Utc::now().format("%Y").to_string());
        self
    }
}

/// Stub Manager for loading and managing stubs
pub struct StubManager {
    stubs: HashMap<String, Stub>,
    stub_paths: Vec<PathBuf>,
}

impl StubManager {
    /// Create a new stub manager
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            stubs: HashMap::new(),
            stub_paths: vec![base_path.as_ref().to_path_buf()],
        }
    }

    /// Add a stub path to search in
    pub fn add_path(&mut self, path: impl AsRef<Path>) {
        self.stub_paths.push(path.as_ref().to_path_buf());
    }

    /// Register a stub
    pub fn register(&mut self, stub: Stub) {
        self.stubs.insert(stub.id.clone(), stub);
    }

    /// Get a stub by ID
    pub fn get(&self, id: &str) -> Result<Stub, StubError> {
        self.stubs
            .get(id)
            .cloned()
            .ok_or_else(|| StubError::NotFound(id.to_string()))
    }

    /// List all available stub IDs
    pub fn list(&self) -> Vec<String> {
        self.stubs.keys().cloned().collect()
    }

    /// Check if a stub exists
    pub fn has(&self, id: &str) -> bool {
        self.stubs.contains_key(id)
    }

    /// Load stubs from filesystem
    pub fn load_from_filesystem(&mut self) -> Result<(), StubError> {
        for path in &self.stub_paths {
            if path.exists() && path.is_dir() {
                self.load_directory(path)?;
            }
        }
        Ok(())
    }

    fn load_directory(&mut self, path: &Path) -> Result<(), StubError> {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(stem) = path.file_stem() {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let id = stem.to_string_lossy().to_string();
                                let extension = path
                                    .extension()
                                    .map(|e| e.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "stub".to_string());

                                let stub = Stub {
                                    id: id.clone(),
                                    name: id.clone(),
                                    description: String::new(),
                                    content,
                                    extension,
                                    builtin: false,
                                    path: Some(path),
                                };

                                self.register(stub);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Get stub count
    pub fn count(&self) -> usize {
        self.stubs.len()
    }

    /// Clear all stubs
    pub fn clear(&mut self) {
        self.stubs.clear();
    }

    /// Get stubs by category (based on ID prefix)
    pub fn by_category(&self, category: &str) -> Vec<Stub> {
        self.stubs
            .values()
            .filter(|stub| stub.id.starts_with(&format!("{}_", category)))
            .cloned()
            .collect()
    }
}

/// Stub-related errors
#[derive(Debug, Clone)]
pub enum StubError {
    /// Stub not found
    NotFound(String),
    /// Error reading stub file
    FileError(String),
    /// Error rendering stub
    RenderError(String),
    /// Invalid stub format
    InvalidFormat(String),
    /// Other errors
    Other(String),
}

impl fmt::Display for StubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StubError::NotFound(id) => write!(f, "Stub '{}' not found", id),
            StubError::FileError(msg) => write!(f, "File error: {}", msg),
            StubError::RenderError(msg) => write!(f, "Render error: {}", msg),
            StubError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            StubError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for StubError {}

/// Built-in stubs for common code generation tasks
pub mod builtins {
    use super::*;

    /// Get the model stub
    pub fn model() -> Stub {
        Stub::new(
            "model",
            "Model",
            r#"<?php

namespace {{namespace}};

use Illuminate\Database\Eloquent\Model;

class {{name}} extends Model
{
    protected $table = '{{table}}';

    protected $fillable = [
        {{fillable}},
    ];
}
"#,
            "php",
        )
        .with_description("Eloquent Model class")
        .builtin()
    }

    /// Get the controller stub
    pub fn controller() -> Stub {
        Stub::new(
            "controller",
            "Controller",
            r#"<?php

namespace {{namespace}};

use Illuminate\Routing\Controller as BaseController;

class {{name}} extends BaseController
{
    //
}
"#,
            "php",
        )
        .with_description("HTTP Controller class")
        .builtin()
    }

    /// Get the migration stub
    pub fn migration() -> Stub {
        Stub::new(
            "migration",
            "Migration",
            r#"<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

class {{name}} extends Migration
{
    public function up()
    {
        Schema::create('{{table}}', function (Blueprint $table) {
            $table->id();
            $table->timestamps();
        });
    }

    public function down()
    {
        Schema::dropIfExists('{{table}}');
    }
}
"#,
            "php",
        )
        .with_description("Database migration")
        .builtin()
    }

    /// Get the job stub
    pub fn job() -> Stub {
        Stub::new(
            "job",
            "Job",
            r#"<?php

namespace {{namespace}};

use Illuminate\Bus\Queueable;
use Illuminate\Queue\SerializesModels;

class {{name}}
{
    use Queueable, SerializesModels;

    public function __construct()
    {
        //
    }

    public function handle()
    {
        //
    }
}
"#,
            "php",
        )
        .with_description("Queueable job class")
        .builtin()
    }

    /// Register all built-in stubs
    pub fn register_all(manager: &mut StubManager) {
        manager.register(model());
        manager.register(controller());
        manager.register(migration());
        manager.register(job());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_creation() {
        let stub = Stub::new("test", "Test Stub", "Content", "rs");
        assert_eq!(stub.id, "test");
        assert_eq!(stub.content, "Content");
    }

    #[test]
    fn test_stub_render() {
        let stub = Stub::new("test", "Test", "Hello {{name}}", "rs");
        let mut vars = StubVariables::new();
        vars.set("name", "World");

        let result = stub.render(&vars).unwrap();
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_stub_placeholders() {
        let stub = Stub::new(
            "test",
            "Test",
            "{{name}}, {{email}}, {{phone}}",
            "rs",
        );
        let placeholders = stub.get_placeholders();
        assert_eq!(placeholders.len(), 3);
        assert!(placeholders.contains(&"name".to_string()));
    }

    #[test]
    fn test_stub_variables() {
        let mut vars = StubVariables::new();
        vars.set("key1", "value1");
        vars.set("key2", "value2");

        assert_eq!(vars.get("key1"), Some("value1"));
        assert!(vars.has("key1"));
        assert!(!vars.has("key3"));
    }

    #[test]
    fn test_stub_manager() {
        let mut manager = StubManager::new(".");
        let stub = Stub::new("test", "Test", "Content", "rs");
        manager.register(stub);

        assert!(manager.has("test"));
        assert_eq!(manager.count(), 1);
        assert!(manager.get("test").is_ok());
        assert!(manager.get("missing").is_err());
    }

    #[test]
    fn test_builtin_stubs() {
        let mut manager = StubManager::new(".");
        builtins::register_all(&mut manager);

        assert!(manager.has("model"));
        assert!(manager.has("controller"));
        assert!(manager.has("migration"));
        assert!(manager.has("job"));
    }

    #[test]
    fn test_variables_merge() {
        let mut vars1 = StubVariables::new();
        vars1.set("key1", "value1");

        let mut vars2 = StubVariables::new();
        vars2.set("key2", "value2");

        let merged = vars1.merge(&vars2);
        assert_eq!(merged.get("key1"), Some("value1"));
        assert_eq!(merged.get("key2"), Some("value2"));
    }
}
