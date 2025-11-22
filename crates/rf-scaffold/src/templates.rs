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

        // Migration template
        handlebars
            .register_template_string("migration", Self::MIGRATION_TEMPLATE)
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

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
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

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
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

    const MIGRATION_TEMPLATE: &'static str = r#"//! {{name}} migration

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table({{table_name}}::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new({{table_name}}::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
{{#each fields}}
                    .col(ColumnDef::new({{../table_name}}::{{this.pascal_name}}){{this.column_def}})
{{/each}}
                    .col(
                        ColumnDef::new({{table_name}}::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new({{table_name}}::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table({{table_name}}::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum {{table_name}} {
    Table,
    Id,
{{#each fields}}
    {{this.pascal_name}},
{{/each}}
    CreatedAt,
    UpdatedAt,
}
"#;

    const SERVICE_TEMPLATE: &'static str = r#"//! {{name}} service

use std::sync::Arc;
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
