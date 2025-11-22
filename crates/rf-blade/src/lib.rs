//! # rf-blade - Laravel Blade-compatible Template Engine
//!
//! A powerful template engine for RustForge that provides Laravel Blade-like syntax
//! with full support for template inheritance, components, and directives.
//!
//! ## Features
//!
//! - **Template Inheritance**: `@extends`, `@section`, `@yield`
//! - **Components**: `<x-component />` syntax
//! - **Directives**: `@if`, `@foreach`, `@auth`, etc.
//! - **Variable Interpolation**: `{{ $variable }}`
//! - **Raw Output**: `{!! $html !!}`
//! - **Comments**: `{{-- comment --}}`
//!
//! ## Quick Start
//!
//! ```rust
//! use rf_blade::BladeEngine;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let blade = BladeEngine::new("templates/")?;
//!
//! let html = blade.render("welcome", json!({
//!     "name": "World",
//!     "items": ["Item 1", "Item 2", "Item 3"]
//! })).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Template Example
//!
//! ```html
//! <!-- templates/layouts/app.blade.html -->
//! <!DOCTYPE html>
//! <html>
//! <head>
//!     <title>@yield('title')</title>
//! </head>
//! <body>
//!     @yield('content')
//! </body>
//! </html>
//!
//! <!-- templates/welcome.blade.html -->
//! @extends('layouts.app')
//!
//! @section('title', 'Welcome')
//!
//! @section('content')
//!     <h1>Hello {{ $name }}!</h1>
//!     @foreach($items as $item)
//!         <p>{{ $item }}</p>
//!     @endforeach
//! @endsection
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::Value;
use thiserror::Error;
use tokio::fs;
use tokio::sync::RwLock;

// Old modules (kept for backwards compatibility)
pub mod parser;
pub mod compiler;
pub mod directives;

// New compiler modules
pub mod lexer;
pub mod ast;
pub mod parser_new;
pub mod compiler_new;

// Phase 2: Components system
pub mod components;

// Phase 19: Stacks system (@push/@stack)
pub mod stacks;

use parser::BladeParser;
use compiler::BladeCompiler;
pub use parser_new::Parser;
pub use compiler_new::{Compiler, RenderContext};

/// Blade template engine errors
#[derive(Error, Debug)]
pub enum BladeError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Render error: {0}")]
    RenderError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type BladeResult<T> = Result<T, BladeError>;

/// Compiled template
#[derive(Debug, Clone)]
pub struct CompiledTemplate {
    /// Template name
    pub name: String,

    /// Compiled HTML
    pub html: String,

    /// Parent template (for @extends)
    pub parent: Option<String>,

    /// Sections defined in this template
    pub sections: HashMap<String, String>,

    /// Components used
    pub components: Vec<String>,
}

/// Blade template engine
///
/// # Example
///
/// ```rust
/// use rf_blade::BladeEngine;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let blade = BladeEngine::new("templates/")?;
///
/// // Register custom directive
/// blade.directive("datetime", |value| {
///     format!("<time>{}</time>", value)
/// })?;
///
/// // Render template
/// let html = blade.render("page", json!({
///     "title": "My Page"
/// })).await?;
/// # Ok(())
/// # }
/// ```
pub struct BladeEngine {
    /// Template base path
    base_path: PathBuf,

    /// Compiled template cache
    cache: Arc<RwLock<HashMap<String, CompiledTemplate>>>,

    /// Custom directives
    directives: Arc<RwLock<HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>>>>,

    /// Component paths
    component_paths: Arc<RwLock<Vec<PathBuf>>>,

    /// Parser
    parser: BladeParser,

    /// Compiler
    compiler: BladeCompiler,
}

impl BladeEngine {
    /// Create a new Blade engine
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_blade::BladeEngine;
    ///
    /// let blade = BladeEngine::new("templates/").unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(base_path: P) -> BladeResult<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        if !base_path.exists() {
            return Err(BladeError::TemplateNotFound(
                format!("Base path does not exist: {}", base_path.display())
            ));
        }

        Ok(Self {
            base_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
            directives: Arc::new(RwLock::new(HashMap::new())),
            component_paths: Arc::new(RwLock::new(vec![])),
            parser: BladeParser::new(),
            compiler: BladeCompiler::new(),
        })
    }

    /// Register a custom directive
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_blade::BladeEngine;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let blade = BladeEngine::new("templates/")?;
    ///
    /// blade.directive("upper", |value| {
    ///     value.to_uppercase()
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn directive<F>(&self, name: &str, handler: F) -> BladeResult<()>
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        let mut directives = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.directives.write())
        });

        directives.insert(name.to_string(), Box::new(handler));
        Ok(())
    }

    /// Add component search path
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_blade::BladeEngine;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let blade = BladeEngine::new("templates/")?;
    /// blade.add_component_path("templates/components/")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_component_path<P: AsRef<Path>>(&self, path: P) -> BladeResult<()> {
        let mut paths = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.component_paths.write())
        });

        paths.push(path.as_ref().to_path_buf());
        Ok(())
    }

    /// Render a template
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_blade::BladeEngine;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let blade = BladeEngine::new("templates/")?;
    ///
    /// let html = blade.render("welcome", json!({
    ///     "name": "Alice",
    ///     "items": vec!["A", "B", "C"]
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render(&self, template: &str, data: Value) -> BladeResult<String> {
        // Load and compile template
        let compiled = self.load_template(template).await?;

        // If template extends another, render parent with sections
        if let Some(parent_name) = &compiled.parent {
            let parent = self.load_template(parent_name).await?;
            self.render_with_parent(&compiled, &parent, data).await
        } else {
            self.render_template(&compiled, data).await
        }
    }

    /// Load and compile a template
    async fn load_template(&self, name: &str) -> BladeResult<CompiledTemplate> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(compiled) = cache.get(name) {
                return Ok(compiled.clone());
            }
        }

        // Load template file
        let template_path = self.resolve_template_path(name)?;
        let content = fs::read_to_string(&template_path).await?;

        // Parse template
        let parsed = self.parser.parse(&content)
            .map_err(|e| BladeError::ParseError(e.to_string()))?;

        // Compile template
        let compiled = self.compiler.compile(name, parsed)?;

        // Cache compiled template
        {
            let mut cache = self.cache.write().await;
            cache.insert(name.to_string(), compiled.clone());
        }

        Ok(compiled)
    }

    /// Resolve template path
    fn resolve_template_path(&self, name: &str) -> BladeResult<PathBuf> {
        // Convert dot notation to path (e.g., "layouts.app" -> "layouts/app")
        let path_str = name.replace('.', "/");

        // Try with .blade.html extension
        let blade_path = self.base_path.join(format!("{}.blade.html", path_str));
        if blade_path.exists() {
            return Ok(blade_path);
        }

        // Try with .html extension
        let html_path = self.base_path.join(format!("{}.html", path_str));
        if html_path.exists() {
            return Ok(html_path);
        }

        Err(BladeError::TemplateNotFound(name.to_string()))
    }

    /// Render template with parent (for @extends)
    async fn render_with_parent(
        &self,
        child: &CompiledTemplate,
        parent: &CompiledTemplate,
        data: Value,
    ) -> BladeResult<String> {
        // Start with parent HTML
        let mut html = parent.html.clone();

        // Replace @yield directives with child sections
        for (section_name, section_content) in &child.sections {
            let yield_marker = format!("@yield('{}')", section_name);
            html = html.replace(&yield_marker, section_content);
        }

        // Interpolate variables
        html = self.interpolate(&html, &data)?;

        Ok(html)
    }

    /// Render a single template
    async fn render_template(
        &self,
        compiled: &CompiledTemplate,
        data: Value,
    ) -> BladeResult<String> {
        let html = self.interpolate(&compiled.html, &data)?;
        Ok(html)
    }

    /// Interpolate variables in HTML
    fn interpolate(&self, html: &str, data: &Value) -> BladeResult<String> {
        let mut result = html.to_string();

        // Simple variable interpolation: {{ $variable }}
        let re = regex::Regex::new(r"\{\{\s*\$?(\w+)\s*\}\}").unwrap();

        result = re.replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];

            if let Some(value) = data.get(var_name) {
                match value {
                    Value::String(s) => html_escape(s),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                }
            } else {
                String::new()
            }
        }).to_string();

        Ok(result)
    }

    /// Clear template cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Render template using the new compiler (REAL compilation)
    ///
    /// This method uses the actual lexer/parser/compiler instead of regex replacement
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_blade::BladeEngine;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let blade = BladeEngine::new("templates/")?;
    ///
    /// let html = blade.render_compiled("@if($show) Hello {{ $name }}! @endif", json!({
    ///     "show": true,
    ///     "name": "Alice"
    /// })).await?;
    ///
    /// assert_eq!(html, " Hello Alice! ");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render_compiled(&self, template: &str, data: Value) -> BladeResult<String> {
        // Parse template into AST
        let ast = Parser::parse(template)
            .map_err(|e| BladeError::ParseError(e.to_string()))?;

        // Create render context
        let mut context = RenderContext::new(data);

        // Compile and execute AST
        let compiler = Compiler::new();
        let html = compiler.compile(&ast, &mut context)
            .map_err(|e| BladeError::RenderError(e.to_string()))?;

        Ok(html)
    }

    /// Render template from file using the new compiler
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_blade::BladeEngine;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let blade = BladeEngine::new("templates/")?;
    ///
    /// let html = blade.render_file_compiled("welcome", json!({
    ///     "name": "World",
    ///     "items": ["A", "B", "C"]
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render_file_compiled(&self, template_name: &str, data: Value) -> BladeResult<String> {
        // Load template file
        let template_path = self.resolve_template_path(template_name)?;
        let content = fs::read_to_string(&template_path).await?;

        // Parse into AST
        let ast = Parser::parse(&content)
            .map_err(|e| BladeError::ParseError(e.to_string()))?;

        // Create render context
        let mut context = RenderContext::new(data);

        // Compile and execute
        let compiler = Compiler::new();
        let html = compiler.compile(&ast, &mut context)
            .map_err(|e| BladeError::RenderError(e.to_string()))?;

        // Handle template inheritance
        if let Some(parent_name) = &context.parent {
            // Load parent template
            let parent_path = self.resolve_template_path(parent_name)?;
            let parent_content = fs::read_to_string(&parent_path).await?;

            // Parse parent
            let parent_ast = Parser::parse(&parent_content)
                .map_err(|e| BladeError::ParseError(e.to_string()))?;

            // Render parent with sections from child
            let parent_html = compiler.compile(&parent_ast, &mut context)
                .map_err(|e| BladeError::RenderError(e.to_string()))?;

            Ok(parent_html)
        } else {
            Ok(html)
        }
    }
}

/// HTML escape for safe output
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }

    #[tokio::test]
    async fn test_interpolation() {
        use std::fs;
        use std::path::PathBuf;

        // Create temporary directory for testing
        let temp_dir = PathBuf::from("/tmp/rf-blade-test");
        fs::create_dir_all(&temp_dir).ok();

        let blade = BladeEngine::new(&temp_dir).unwrap();

        let html = blade.interpolate(
            "Hello {{ $name }}!",
            &json!({"name": "World"})
        ).unwrap();

        assert_eq!(html, "Hello World!");

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_interpolation_escape() {
        use std::fs;
        use std::path::PathBuf;

        // Create temporary directory for testing
        let temp_dir = PathBuf::from("/tmp/rf-blade-test-2");
        fs::create_dir_all(&temp_dir).ok();

        let blade = BladeEngine::new(&temp_dir).unwrap();

        let html = blade.interpolate(
            "{{ $code }}",
            &json!({"code": "<script>alert('xss')</script>"})
        ).unwrap();

        assert_eq!(html, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");

        // Cleanup
        fs::remove_dir_all(&temp_dir).ok();
    }
}
