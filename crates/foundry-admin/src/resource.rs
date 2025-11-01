//! Admin resource abstraction for CRUD operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Configuration for an admin resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub name: String,
    pub label: String,
    pub icon: Option<String>,
    pub fields: Vec<FieldConfig>,
    pub searchable_fields: Vec<String>,
    pub filterable_fields: Vec<String>,
    pub sortable_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub readonly: bool,
    pub help_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Email,
    Password,
    Number,
    Boolean,
    Date,
    DateTime,
    TextArea,
    Select { options: Vec<SelectOption> },
    File,
    Image,
    Relation { resource: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
    pub page: usize,
    pub per_page: usize,
    pub search: Option<String>,
    pub filters: HashMap<String, String>,
    pub sort_by: Option<String>,
    pub sort_desc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub data: Vec<Value>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Trait for admin resources with CRUD operations
#[async_trait]
pub trait AdminResource: Send + Sync {
    /// Get resource configuration
    fn config(&self) -> &ResourceConfig;

    /// List records with pagination and filters
    async fn list(&self, query: ListQuery) -> anyhow::Result<ListResult>;

    /// Get a single record by ID
    async fn get(&self, id: &str) -> anyhow::Result<Option<Value>>;

    /// Create a new record
    async fn create(&self, data: Value) -> anyhow::Result<Value>;

    /// Update an existing record
    async fn update(&self, id: &str, data: Value) -> anyhow::Result<Value>;

    /// Delete a record
    async fn delete(&self, id: &str) -> anyhow::Result<()>;

    /// Validate input data
    async fn validate(&self, data: &Value, is_update: bool) -> anyhow::Result<ValidationResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: HashMap<String, Vec<String>>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: HashMap::new(),
        }
    }

    pub fn with_error(mut self, field: impl Into<String>, error: impl Into<String>) -> Self {
        self.valid = false;
        self.errors
            .entry(field.into())
            .or_default()
            .push(error.into());
        self
    }
}

/// Trait for CRUD operations on any entity
#[async_trait]
pub trait CrudOperations<T>
where
    T: Send + Sync,
{
    async fn list(&self, query: ListQuery) -> anyhow::Result<Vec<T>>;
    async fn count(&self, query: &ListQuery) -> anyhow::Result<usize>;
    async fn find(&self, id: &str) -> anyhow::Result<Option<T>>;
    async fn create(&self, entity: T) -> anyhow::Result<T>;
    async fn update(&self, id: &str, entity: T) -> anyhow::Result<T>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
}
