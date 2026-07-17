//! Built-in templates for code generation

use crate::ScaffoldResult;
use handlebars::Handlebars;

/// Built-in template definitions
pub struct BuiltinTemplates;

impl BuiltinTemplates {
    /// Register all built-in templates
    pub fn register(handlebars: &mut Handlebars<'static>) -> ScaffoldResult<()> {
        // Model template
        handlebars
            .register_template_string("model", Self::MODEL_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Controller template
        handlebars
            .register_template_string("controller", Self::CONTROLLER_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Resource controller template
        handlebars
            .register_template_string("controller_resource", Self::CONTROLLER_RESOURCE_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Canonical plain-SQL migration templates (up.sql / down.sql per migration directory).
        // All three generators (rf-scaffold, forge-cli, foundry-cli) emit this same format.
        // Runner: DB::statement(include_str!("up.sql")).expect("migration failed");
        handlebars
            .register_template_string("migration_up", Self::MIGRATION_UP_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;
        handlebars
            .register_template_string("migration_down", Self::MIGRATION_DOWN_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;
        handlebars
            .register_template_string("migration_model_up", Self::MIGRATION_MODEL_UP_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;
        handlebars
            .register_template_string("migration_model_down", Self::MIGRATION_MODEL_DOWN_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Service template
        handlebars
            .register_template_string("service", Self::SERVICE_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Repository template
        handlebars
            .register_template_string("repository", Self::REPOSITORY_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        // Test template
        handlebars
            .register_template_string("test", Self::TEST_TEMPLATE)
            .map_err(|e| crate::ScaffoldError::RenderError(e.to_string()))?;

        Ok(())
    }

    const MODEL_TEMPLATE: &'static str = r#"//! {{name}} model
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// {{name}} model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{name}} {
    /// Primary key
    pub id: i64,

{{#each fields}}
    /// {{this.name}} field
    pub {{this.name}}: {{this.field_type}},

{{/each}}
    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl {{name}} {
    /// Create a new {{name}}
    pub fn new({{#each fields}}{{this.name}}: {{this.field_type}}{{#unless @last}}, {{/unless}}{{/each}}) -> Self {
        Self {
            id: 0,
{{#each fields}}
            {{this.name}},
{{/each}}
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Update timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_{{snake_name}}() {
        let {{snake_name}} = {{name}}::new({{#each fields}}{{#if (eq this.field_type "String")}}"test".to_string(){{/if}}{{#if (eq this.field_type "i32")}}42{{/if}}{{#if (eq this.field_type "i64")}}42{{/if}}{{#if (eq this.field_type "bool")}}true{{/if}}{{#unless @last}}, {{/unless}}{{/each}});
        assert_eq!({{snake_name}}.id, 0);
    }
}
"#;

    const CONTROLLER_TEMPLATE: &'static str = r#"//! {{name}} controller
#![allow(dead_code)]

use axum::{
    extract::{Path, Json},
    response::IntoResponse,
};
use serde_json::json;

/// {{name}} controller
pub struct {{name}};

impl {{name}} {
    /// Create new controller instance
    pub fn new() -> Self {
        Self
    }

    /// Handle index request
    pub async fn index() -> impl IntoResponse {
        Json(json!({
            "message": "{{name}} index"
        }))
    }

    /// Handle show request
    pub async fn show(Path(id): Path<i64>) -> impl IntoResponse {
        Json(json!({
            "id": id,
            "message": "Show {{name}}"
        }))
    }
}

impl Default for {{name}} {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_controller() {
        let controller = {{name}}::new();
        // Add tests
    }
}
"#;

    const CONTROLLER_RESOURCE_TEMPLATE: &'static str = r#"//! {{name}} resource controller
#![allow(dead_code)]

use axum::{
    extract::{Path, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;

/// Create {{model_name}} request
#[derive(Debug, Deserialize)]
pub struct Create{{model_name}}Request {
{{#each fields}}
    pub {{this.name}}: {{this.field_type}},
{{/each}}
}

/// Update {{model_name}} request
#[derive(Debug, Deserialize)]
pub struct Update{{model_name}}Request {
{{#each fields}}
    pub {{this.name}}: Option<{{this.field_type}}>,
{{/each}}
}

/// {{name}} resource controller
pub struct {{name}};

impl {{name}} {
    /// List all {{plural_model}}
    pub async fn index() -> impl IntoResponse {
        Json(json!({
            "data": [],
            "message": "List of {{plural_model}}"
        }))
    }

    /// Create new {{model_name}}
    pub async fn store(
        Json(payload): Json<Create{{model_name}}Request>
    ) -> impl IntoResponse {
        // TODO: Implement create logic
        (StatusCode::CREATED, Json(json!({
            "message": "{{model_name}} created"
        })))
    }

    /// Show specific {{model_name}}
    pub async fn show(Path(id): Path<i64>) -> impl IntoResponse {
        Json(json!({
            "id": id,
            "message": "Show {{model_name}}"
        }))
    }

    /// Update {{model_name}}
    pub async fn update(
        Path(id): Path<i64>,
        Json(payload): Json<Update{{model_name}}Request>
    ) -> impl IntoResponse {
        Json(json!({
            "id": id,
            "message": "{{model_name}} updated"
        }))
    }

    /// Delete {{model_name}}
    pub async fn destroy(Path(id): Path<i64>) -> impl IntoResponse {
        (StatusCode::NO_CONTENT, Json(json!({
            "message": "{{model_name}} deleted"
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller() {
        // Add tests
    }
}
"#;

    // ---------------------------------------------------------------------------
    // Canonical plain-SQL migration templates
    //
    // All generators (rf-scaffold, forge-cli, foundry-cli) now emit the same
    // convention: a timestamped directory holding up.sql and down.sql.
    //
    // Canonical runner (the only migration path that runs against the real DB):
    //   DB::statement(include_str!("up.sql")).expect("migration failed");
    //   or via `rf migrate` / foundry `make:migration --run`.
    // ---------------------------------------------------------------------------

    /// Standalone migration skeleton — up.sql (no model fields; developer fills in SQL).
    const MIGRATION_UP_TEMPLATE: &'static str = r#"-- Up migration: {{name}}
-- Canonical RustForge plain-SQL migration.
-- Runner: DB::statement(include_str!("up.sql")).expect("migration failed");
--
-- TODO: Write your migration SQL here. Example:
-- CREATE TABLE IF NOT EXISTS my_table (
--     id INTEGER PRIMARY KEY AUTOINCREMENT,
--     name TEXT NOT NULL,
--     created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
--     updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
-- );
"#;

    /// Standalone migration skeleton — down.sql.
    const MIGRATION_DOWN_TEMPLATE: &'static str = r#"-- Down migration: {{name}}
-- Reverse of up.sql.
--
-- TODO: Write the rollback SQL here. Example:
-- DROP TABLE IF EXISTS my_table;
"#;

    /// Model migration up.sql — emits a real CREATE TABLE with the model's fields.
    /// Field columns are rendered from the `fields` array (each has `name` + `sql_type`).
    const MIGRATION_MODEL_UP_TEMPLATE: &'static str = r#"-- Up migration: create {{table_sql_name}} table
-- Canonical RustForge plain-SQL migration.
-- Runner: DB::statement(include_str!("up.sql")).expect("migration failed");

CREATE TABLE IF NOT EXISTS {{table_sql_name}} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
{{#each fields}}    {{this.name}} {{this.sql_type}},
{{/each}}    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
"#;

    /// Model migration down.sql — drops the table created in up.sql.
    const MIGRATION_MODEL_DOWN_TEMPLATE: &'static str = r#"-- Down migration: drop {{table_sql_name}} table
DROP TABLE IF EXISTS {{table_sql_name}};
"#;

    const SERVICE_TEMPLATE: &'static str = r#"//! {{name}} service
#![allow(dead_code)]

use async_trait::async_trait;
use anyhow::Result;

/// {{name}} service trait
#[async_trait]
pub trait {{name}}Trait: Send + Sync {
    /// Execute service logic
    async fn execute(&self) -> Result<()>;
}

/// {{name}} implementation
pub struct {{name}} {
    // Add dependencies here
}

impl {{name}} {
    /// Create new service instance
    pub fn new() -> Self {
        Self {
            // Initialize dependencies
        }
    }
}

impl Default for {{name}} {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl {{name}}Trait for {{name}} {
    async fn execute(&self) -> Result<()> {
        // Implement service logic
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_execute() {
        let service = {{name}}::new();
        let result = service.execute().await;
        assert!(result.is_ok());
    }
}
"#;

    const REPOSITORY_TEMPLATE: &'static str = r#"//! {{name}} repository
#![allow(dead_code)]

use async_trait::async_trait;
use anyhow::Result;

/// {{name}} repository trait
#[async_trait]
pub trait {{name}}Trait: Send + Sync {
    /// Find by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<{{model_name}}>>;

    /// Find all
    async fn find_all(&self) -> Result<Vec<{{model_name}}>>;

    /// Create
    async fn create(&self, data: &{{model_name}}) -> Result<{{model_name}}>;

    /// Update
    async fn update(&self, id: i64, data: &{{model_name}}) -> Result<{{model_name}}>;

    /// Delete
    async fn delete(&self, id: i64) -> Result<()>;
}

/// {{name}} implementation
pub struct {{name}} {
    // Database connection or pool
}

impl {{name}} {
    /// Create new repository instance
    pub fn new() -> Self {
        Self {
            // Initialize connection
        }
    }
}

impl Default for {{name}} {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl {{name}}Trait for {{name}} {
    async fn find_by_id(&self, id: i64) -> Result<Option<{{model_name}}>> {
        // TODO: Implement database query
        Ok(None)
    }

    async fn find_all(&self) -> Result<Vec<{{model_name}}>> {
        // TODO: Implement database query
        Ok(vec![])
    }

    async fn create(&self, data: &{{model_name}}) -> Result<{{model_name}}> {
        // TODO: Implement database insert
        Ok(data.clone())
    }

    async fn update(&self, id: i64, data: &{{model_name}}) -> Result<{{model_name}}> {
        // TODO: Implement database update
        Ok(data.clone())
    }

    async fn delete(&self, id: i64) -> Result<()> {
        // TODO: Implement database delete
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repository() {
        let repo = {{name}}::new();
        // Add tests
    }
}
"#;

    const TEST_TEMPLATE: &'static str = r#"//! {{name}} tests

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_{{snake_name}}() {
        // TODO: Add test implementation
        assert!(true);
    }

    #[tokio::test]
    async fn test_{{snake_name}}_async() {
        // TODO: Add async test implementation
        assert!(true);
    }
}
"#;
}
