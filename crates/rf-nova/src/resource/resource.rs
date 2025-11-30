//! Core Resource trait and implementation
//!
//! Resources are the central concept in Nova - they map models to admin panel interfaces.

use super::field::{Field, FieldContext};
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Result type for resource operations
pub type ResourceResult<T> = Result<T, ResourceError>;

/// Resource errors
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Core Resource trait
#[async_trait]
pub trait Resource: Send + Sync {
    /// The SeaORM entity type this resource represents
    type Entity: EntityTrait;

    /// The model type
    type Model: ModelTrait + Serialize + Send + Sync;

    /// Get resource name (singular)
    fn name() -> &'static str;

    /// Get resource plural name
    fn plural() -> &'static str {
        Self::name()  // Override this method to provide custom plural
    }

    /// Get resource group (for sidebar navigation)
    fn group() -> Option<&'static str> {
        None
    }

    /// Get resource label (human-readable singular)
    fn label() -> &'static str {
        Self::name()
    }

    /// Get resource plural label
    fn plural_label() -> &'static str {
        Self::plural()
    }

    /// Get the fields for this resource
    fn fields() -> Vec<Box<dyn Field>>;

    /// Get fields for a specific context
    fn fields_for_context(context: FieldContext) -> Vec<Box<dyn Field>> {
        Self::fields()
            .into_iter()
            .filter(|f| f.is_visible(context))
            .collect()
    }

    /// Get searchable field names
    fn searchable_fields() -> Vec<String> {
        Self::fields()
            .iter()
            .filter(|f| f.is_searchable())
            .map(|f| f.name().to_string())
            .collect()
    }

    /// Get sortable field names
    fn sortable_fields() -> Vec<String> {
        Self::fields()
            .iter()
            .filter(|f| f.is_sortable())
            .map(|f| f.name().to_string())
            .collect()
    }

    /// Get the title attribute for display
    fn title_attribute() -> &'static str {
        "id"
    }

    /// Get subtitle attributes for display
    fn subtitle_attributes() -> Vec<&'static str> {
        vec![]
    }

    /// Maximum items per page
    fn per_page_options() -> Vec<u64> {
        vec![15, 25, 50, 100]
    }

    /// Default items per page
    fn default_per_page() -> u64 {
        15
    }

    /// Check if resource is globally searchable
    fn globally_searchable() -> bool {
        true
    }

    /// Serialize resource schema for JSON API
    fn to_schema() -> ResourceSchema {
        ResourceSchema {
            name: Self::name().to_string(),
            plural: Self::plural().to_string(),
            label: Self::label().to_string(),
            plural_label: Self::plural_label().to_string(),
            group: Self::group().map(String::from),
            fields: Self::fields().iter().map(|f| f.to_json()).collect(),
            searchable_fields: Self::searchable_fields(),
            sortable_fields: Self::sortable_fields(),
            per_page_options: Self::per_page_options(),
            default_per_page: Self::default_per_page(),
        }
    }

    /// Get resource actions
    fn actions() -> Vec<Box<dyn crate::action::Action>> {
        vec![]
    }

    /// Get resource filters
    fn filters() -> Vec<Box<dyn crate::filter::Filter>> {
        vec![]
    }

    /// Get resource lenses
    fn lenses() -> Vec<Box<dyn crate::lens::Lens>> {
        vec![]
    }

    /// Get resource cards (for detail page)
    fn cards() -> Vec<Box<dyn crate::card::Card>> {
        vec![]
    }

    /// Authorization - can view any records
    async fn authorize_view_any(_user: Option<&Value>) -> bool {
        true
    }

    /// Authorization - can view specific record
    async fn authorize_view(_user: Option<&Value>, _model: &Self::Model) -> bool {
        true
    }

    /// Authorization - can create records
    async fn authorize_create(_user: Option<&Value>) -> bool {
        true
    }

    /// Authorization - can update specific record
    async fn authorize_update(_user: Option<&Value>, _model: &Self::Model) -> bool {
        true
    }

    /// Authorization - can delete specific record
    async fn authorize_delete(_user: Option<&Value>, _model: &Self::Model) -> bool {
        true
    }

    /// Hook called before creating a record
    async fn before_create(_data: &mut HashMap<String, Value>) -> ResourceResult<()> {
        Ok(())
    }

    /// Hook called after creating a record
    async fn after_create(_model: &Self::Model) -> ResourceResult<()> {
        Ok(())
    }

    /// Hook called before updating a record
    async fn before_update(_model: &Self::Model, _data: &mut HashMap<String, Value>) -> ResourceResult<()> {
        Ok(())
    }

    /// Hook called after updating a record
    async fn after_update(_model: &Self::Model) -> ResourceResult<()> {
        Ok(())
    }

    /// Hook called before deleting a record
    async fn before_delete(_model: &Self::Model) -> ResourceResult<()> {
        Ok(())
    }

    /// Hook called after deleting a record
    async fn after_delete(_model_id: Value) -> ResourceResult<()> {
        Ok(())
    }
}

/// Resource schema for JSON serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSchema {
    pub name: String,
    pub plural: String,
    pub label: String,
    pub plural_label: String,
    pub group: Option<String>,
    pub fields: Vec<Value>,
    pub searchable_fields: Vec<String>,
    pub sortable_fields: Vec<String>,
    pub per_page_options: Vec<u64>,
    pub default_per_page: u64,
}

/// Resource query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuery {
    /// Current page number
    pub page: Option<u64>,

    /// Items per page
    pub per_page: Option<u64>,

    /// Search query
    pub search: Option<String>,

    /// Sort field
    pub sort_by: Option<String>,

    /// Sort direction (asc/desc)
    pub sort_order: Option<String>,

    /// Filters
    pub filters: Option<HashMap<String, String>>,
}

impl Default for ResourceQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            per_page: Some(15),
            search: None,
            sort_by: None,
            sort_order: Some("asc".to_string()),
            filters: None,
        }
    }
}

/// Paginated resource response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub current_page: u64,
    pub per_page: u64,
    pub total: u64,
    pub last_page: u64,
    pub from: Option<u64>,
    pub to: Option<u64>,
}

impl PaginationMeta {
    pub fn new(current_page: u64, per_page: u64, total: u64) -> Self {
        let last_page = (total as f64 / per_page as f64).ceil() as u64;
        let from = if total > 0 {
            Some((current_page - 1) * per_page + 1)
        } else {
            None
        };
        let to = if total > 0 {
            Some(std::cmp::min(current_page * per_page, total))
        } else {
            None
        };

        Self {
            current_page,
            per_page,
            total,
            last_page,
            from,
            to,
        }
    }
}
