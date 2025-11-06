use serde::{Deserialize, Serialize};

/// Type of stub/template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StubType {
    Model,
    Controller,
    Migration,
    Service,
    Repository,
    Request,
    Response,
    Test,
    Custom,
}

impl StubType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Controller => "controller",
            Self::Migration => "migration",
            Self::Service => "service",
            Self::Repository => "repository",
            Self::Request => "request",
            Self::Response => "response",
            Self::Test => "test",
            Self::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "model" => Some(Self::Model),
            "controller" => Some(Self::Controller),
            "migration" => Some(Self::Migration),
            "service" => Some(Self::Service),
            "repository" => Some(Self::Repository),
            "request" => Some(Self::Request),
            "response" => Some(Self::Response),
            "test" => Some(Self::Test),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

/// A stub template
#[derive(Debug, Clone)]
pub struct Stub {
    pub name: String,
    pub stub_type: StubType,
    pub content: String,
    pub is_custom: bool,
}

impl Stub {
    pub fn new(name: impl Into<String>, stub_type: StubType, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stub_type,
            content: content.into(),
            is_custom: false,
        }
    }

    pub fn custom(mut self) -> Self {
        self.is_custom = true;
        self
    }
}

/// Default stub templates
pub struct DefaultStubs;

impl DefaultStubs {
    pub fn model() -> &'static str {
        r#"use serde::{Deserialize, Serialize};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{{ snake_plural }}")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
{{ properties }}
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#
    }

    pub fn controller() -> &'static str {
        r#"use axum::{extract::Path, response::Json, Extension};
use std::sync::Arc;

use crate::app::models::{{ snake }}::{{ studly }};
use crate::error::AppError;

pub struct {{ studly }}Controller;

impl {{ studly }}Controller {
    pub async fn index() -> Result<Json<Vec<{{ studly }}>>, AppError> {
        // List all {{ plural }}
        todo!("Implement index")
    }

    pub async fn show(Path(id): Path<i64>) -> Result<Json<{{ studly }}>, AppError> {
        // Show single {{ singular }}
        todo!("Implement show")
    }

    pub async fn store(Json(data): Json<Create{{ studly }}Request>) -> Result<Json<{{ studly }}>, AppError> {
        // Create new {{ singular }}
        todo!("Implement store")
    }

    pub async fn update(
        Path(id): Path<i64>,
        Json(data): Json<Update{{ studly }}Request>,
    ) -> Result<Json<{{ studly }}>, AppError> {
        // Update {{ singular }}
        todo!("Implement update")
    }

    pub async fn destroy(Path(id): Path<i64>) -> Result<Json<()>, AppError> {
        // Delete {{ singular }}
        todo!("Implement destroy")
    }
}
"#
    }

    pub fn service() -> &'static str {
        r#"use std::sync::Arc;
use crate::app::models::{{ snake }}::{{ studly }};
use crate::error::AppError;

pub struct {{ studly }}Service;

impl {{ studly }}Service {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_all(&self) -> Result<Vec<{{ studly }}>, AppError> {
        todo!("Implement get_all")
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<{{ studly }}>, AppError> {
        todo!("Implement get_by_id")
    }

    pub async fn create(&self, data: Create{{ studly }}Data) -> Result<{{ studly }}, AppError> {
        todo!("Implement create")
    }

    pub async fn update(&self, id: i64, data: Update{{ studly }}Data) -> Result<{{ studly }}, AppError> {
        todo!("Implement update")
    }

    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        todo!("Implement delete")
    }
}
"#
    }

    pub fn migration() -> &'static str {
        r#"use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table({{ studly }}::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new({{ studly }}::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
{{ columns }}
                    .col(ColumnDef::new({{ studly }}::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new({{ studly }}::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table({{ studly }}::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum {{ studly }} {
    Table,
    Id,
{{ column_idents }}
    CreatedAt,
    UpdatedAt,
}
"#
    }

    pub fn test() -> &'static str {
        r#"#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_{{ snake }}_creation() {
        todo!("Implement test")
    }

    #[tokio::test]
    async fn test_{{ snake }}_update() {
        todo!("Implement test")
    }

    #[tokio::test]
    async fn test_{{ snake }}_deletion() {
        todo!("Implement test")
    }
}
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_type_conversion() {
        assert_eq!(StubType::Model.as_str(), "model");
        assert_eq!(StubType::from_str("controller"), Some(StubType::Controller));
        assert_eq!(StubType::from_str("invalid"), None);
    }

    #[test]
    fn test_stub_creation() {
        let stub = Stub::new("user", StubType::Model, "content");
        assert_eq!(stub.name, "user");
        assert_eq!(stub.stub_type, StubType::Model);
        assert!(!stub.is_custom);
    }

    #[test]
    fn test_custom_stub() {
        let stub = Stub::new("user", StubType::Model, "content").custom();
        assert!(stub.is_custom);
    }
}
