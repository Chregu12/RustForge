//! Admin Panel Generator for RustForge
//!
//! This crate provides automatic CRUD interface generation.

use async_trait::async_trait;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use thiserror::Error;

/// Admin errors
#[derive(Debug, Error)]
pub enum AdminError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Authorization error: {0}")]
    AuthorizationError(String),
}

pub type AdminResult<T> = Result<T, AdminError>;

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        let status = match self {
            AdminError::ResourceNotFound(_) => StatusCode::NOT_FOUND,
            AdminError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AdminError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AdminError::AuthorizationError(_) => StatusCode::FORBIDDEN,
        };

        (status, self.to_string()).into_response()
    }
}

/// Field configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    pub name: String,
    pub label: String,
    pub field_type: FieldType,
    pub required: bool,
    pub searchable: bool,
    pub sortable: bool,
    pub list_display: bool,
}

impl FieldConfig {
    pub fn new(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            field_type: FieldType::Text,
            required: false,
            searchable: false,
            sortable: false,
            list_display: true,
        }
    }

    pub fn field_type(mut self, field_type: FieldType) -> Self {
        self.field_type = field_type;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn list_display(mut self, display: bool) -> Self {
        self.list_display = display;
        self
    }
}

/// Field types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    Email,
    Password,
    Number,
    Date,
    DateTime,
    Boolean,
    Select(Vec<String>),
    TextArea,
}

/// List query parameters
#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
}

/// Admin resource trait
#[async_trait]
pub trait AdminResource: Send + Sync + 'static {
    /// Get resource name
    fn name(&self) -> &str;

    /// Get resource label (for display)
    fn label(&self) -> &str;

    /// Get field configurations
    fn fields(&self) -> Vec<FieldConfig>;

    /// List resources
    async fn list(&self, params: ListParams) -> AdminResult<AdminList>;

    /// Get a single resource
    async fn get(&self, id: &str) -> AdminResult<serde_json::Value>;

    /// Create a resource
    async fn create(&self, data: serde_json::Value) -> AdminResult<serde_json::Value>;

    /// Update a resource
    async fn update(&self, id: &str, data: serde_json::Value) -> AdminResult<serde_json::Value>;

    /// Delete a resource
    async fn delete(&self, id: &str) -> AdminResult<()>;

    /// Get menu group (for organizing resources)
    fn menu_group(&self) -> Option<&str> {
        None
    }

    /// Get icon (for display)
    fn icon(&self) -> Option<&str> {
        None
    }
}

/// List response
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminList {
    pub data: Vec<serde_json::Value>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub last_page: u32,
}

impl AdminList {
    pub fn new(data: Vec<serde_json::Value>, total: u64, page: u32, per_page: u32) -> Self {
        let last_page = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            data,
            total,
            page,
            per_page,
            last_page,
        }
    }
}

/// Admin panel
pub struct AdminPanel {
    title: String,
    resources: HashMap<String, Arc<dyn AdminResource>>,
}

impl AdminPanel {
    /// Create a new admin panel
    pub fn new() -> Self {
        Self {
            title: "Admin Panel".to_string(),
            resources: HashMap::new(),
        }
    }

    /// Set panel title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Register a resource
    pub fn resource(mut self, resource: Arc<dyn AdminResource>) -> Self {
        self.resources.insert(resource.name().to_string(), resource);
        self
    }

    /// Build the admin panel router
    pub fn build(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/", get(index_handler))
            .route("/resources", get(resources_handler))
            .route("/resources/:resource", get(resource_list_handler))
            .route("/resources/:resource/create", get(resource_create_form_handler))
            .route("/resources/:resource", post(resource_create_handler))
            .route("/resources/:resource/:id", get(resource_show_handler))
            .route("/resources/:resource/:id/edit", get(resource_edit_form_handler))
            .route("/resources/:resource/:id", post(resource_update_handler))
            .route("/resources/:resource/:id/delete", post(resource_delete_handler))
            .with_state(state)
    }
}

impl Default for AdminPanel {
    fn default() -> Self {
        Self::new()
    }
}

// Handler functions
async fn index_handler(
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> impl IntoResponse {
    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>{}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; }}
        h1 {{ color: #333; }}
        .resource {{ margin: 10px 0; padding: 10px; border: 1px solid #ddd; }}
        .resource a {{ text-decoration: none; color: #0066cc; }}
    </style>
</head>
<body>
    <h1>{}</h1>
    <div>
        {}
    </div>
</body>
</html>"#,
        panel.title,
        panel.title,
        panel
            .resources
            .values()
            .map(|r| format!(
                r#"<div class="resource"><a href="/resources/{}">{}</a></div>"#,
                r.name(),
                r.label()
            ))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

async fn resources_handler(
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> impl IntoResponse {
    let resources: Vec<_> = panel
        .resources
        .values()
        .map(|r| {
            serde_json::json!({
                "name": r.name(),
                "label": r.label(),
                "menu_group": r.menu_group(),
                "icon": r.icon(),
            })
        })
        .collect();

    Json(resources)
}

async fn resource_list_handler(
    Path(resource_name): Path<String>,
    Query(params): Query<ListParams>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let list = resource.list(params).await?;
    Ok(Json(list))
}

async fn resource_show_handler(
    Path((resource_name, id)): Path<(String, String)>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let data = resource.get(&id).await?;
    Ok(Json(data))
}

async fn resource_create_form_handler(
    Path(resource_name): Path<String>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let fields = resource.fields();
    Ok(Json(fields))
}

async fn resource_create_handler(
    Path(resource_name): Path<String>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
    Json(data): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let created = resource.create(data).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn resource_edit_form_handler(
    Path((resource_name, id)): Path<(String, String)>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let data = resource.get(&id).await?;
    let fields = resource.fields();

    Ok(Json(serde_json::json!({
        "data": data,
        "fields": fields,
    })))
}

async fn resource_update_handler(
    Path((resource_name, id)): Path<(String, String)>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
    Json(data): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    let updated = resource.update(&id, data).await?;
    Ok(Json(updated))
}

async fn resource_delete_handler(
    Path((resource_name, id)): Path<(String, String)>,
    axum::extract::State(panel): axum::extract::State<Arc<AdminPanel>>,
) -> Result<impl IntoResponse, AdminError> {
    let resource = panel
        .resources
        .get(&resource_name)
        .ok_or_else(|| AdminError::ResourceNotFound(resource_name.clone()))?;

    resource.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestResource;

    #[async_trait]
    impl AdminResource for TestResource {
        fn name(&self) -> &str {
            "users"
        }

        fn label(&self) -> &str {
            "Users"
        }

        fn fields(&self) -> Vec<FieldConfig> {
            vec![
                FieldConfig::new("id", "ID")
                    .field_type(FieldType::Number)
                    .sortable(),
                FieldConfig::new("name", "Name")
                    .required()
                    .searchable()
                    .sortable(),
                FieldConfig::new("email", "Email")
                    .field_type(FieldType::Email)
                    .required()
                    .searchable(),
            ]
        }

        async fn list(&self, _params: ListParams) -> AdminResult<AdminList> {
            Ok(AdminList::new(
                vec![
                    serde_json::json!({"id": 1, "name": "Alice", "email": "alice@example.com"}),
                    serde_json::json!({"id": 2, "name": "Bob", "email": "bob@example.com"}),
                ],
                2,
                1,
                10,
            ))
        }

        async fn get(&self, id: &str) -> AdminResult<serde_json::Value> {
            if id == "1" {
                Ok(serde_json::json!({"id": 1, "name": "Alice", "email": "alice@example.com"}))
            } else {
                Err(AdminError::ResourceNotFound(id.to_string()))
            }
        }

        async fn create(&self, data: serde_json::Value) -> AdminResult<serde_json::Value> {
            Ok(serde_json::json!({
                "id": 3,
                "name": data["name"],
                "email": data["email"]
            }))
        }

        async fn update(&self, id: &str, data: serde_json::Value) -> AdminResult<serde_json::Value> {
            Ok(serde_json::json!({
                "id": id.parse::<i64>().unwrap(),
                "name": data["name"],
                "email": data["email"]
            }))
        }

        async fn delete(&self, _id: &str) -> AdminResult<()> {
            Ok(())
        }

        fn menu_group(&self) -> Option<&str> {
            Some("User Management")
        }

        fn icon(&self) -> Option<&str> {
            Some("user")
        }
    }

    #[test]
    fn test_field_config_builder() {
        let field = FieldConfig::new("email", "Email Address")
            .field_type(FieldType::Email)
            .required()
            .searchable();

        assert_eq!(field.name, "email");
        assert_eq!(field.label, "Email Address");
        assert!(field.required);
        assert!(field.searchable);
        assert!(matches!(field.field_type, FieldType::Email));
    }

    #[test]
    fn test_admin_list_last_page_calculation() {
        let list = AdminList::new(vec![], 25, 1, 10);
        assert_eq!(list.last_page, 3);

        let list = AdminList::new(vec![], 30, 1, 10);
        assert_eq!(list.last_page, 3);

        let list = AdminList::new(vec![], 31, 1, 10);
        assert_eq!(list.last_page, 4);
    }

    #[test]
    fn test_admin_panel_creation() {
        let panel = AdminPanel::new()
            .title("My Admin")
            .resource(Arc::new(TestResource));

        assert_eq!(panel.title, "My Admin");
        assert_eq!(panel.resources.len(), 1);
        assert!(panel.resources.contains_key("users"));
    }

    #[tokio::test]
    async fn test_resource_list() {
        let resource = TestResource;
        let params = ListParams {
            page: Some(1),
            per_page: Some(10),
            search: None,
            sort: None,
            order: None,
        };

        let list = resource.list(params).await.unwrap();
        assert_eq!(list.data.len(), 2);
        assert_eq!(list.total, 2);
    }

    #[tokio::test]
    async fn test_resource_get() {
        let resource = TestResource;
        let data = resource.get("1").await.unwrap();
        assert_eq!(data["name"], "Alice");
        assert_eq!(data["email"], "alice@example.com");
    }

    #[tokio::test]
    async fn test_resource_get_not_found() {
        let resource = TestResource;
        let result = resource.get("999").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resource_create() {
        let resource = TestResource;
        let data = serde_json::json!({
            "name": "Charlie",
            "email": "charlie@example.com"
        });

        let created = resource.create(data).await.unwrap();
        assert_eq!(created["name"], "Charlie");
        assert_eq!(created["email"], "charlie@example.com");
    }

    #[tokio::test]
    async fn test_resource_update() {
        let resource = TestResource;
        let data = serde_json::json!({
            "name": "Alice Updated",
            "email": "alice.new@example.com"
        });

        let updated = resource.update("1", data).await.unwrap();
        assert_eq!(updated["name"], "Alice Updated");
    }

    #[tokio::test]
    async fn test_resource_delete() {
        let resource = TestResource;
        let result = resource.delete("1").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_metadata() {
        let resource = TestResource;
        assert_eq!(resource.name(), "users");
        assert_eq!(resource.label(), "Users");
        assert_eq!(resource.menu_group(), Some("User Management"));
        assert_eq!(resource.icon(), Some("user"));
    }

    #[test]
    fn test_field_types() {
        let text = FieldType::Text;
        let email = FieldType::Email;
        let select = FieldType::Select(vec!["Option 1".to_string(), "Option 2".to_string()]);

        assert!(matches!(text, FieldType::Text));
        assert!(matches!(email, FieldType::Email));
        assert!(matches!(select, FieldType::Select(_)));
    }
}
