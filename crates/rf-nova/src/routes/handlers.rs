//! Route handlers for Nova API
//!
//! Handles all HTTP requests for the Nova admin panel.

use crate::Nova;
use crate::routes::api::{
    ApiResponse, ConfigResponse, DashboardInfo, DashboardListResponse, ResourceInfo,
    ResourceListResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// List all registered resources
pub async fn list_resources(
    State(nova): State<Arc<Nova>>,
) -> Result<Json<ApiResponse<ResourceListResponse>>, StatusCode> {
    let resources: Vec<ResourceInfo> = nova
        .resources()
        .values()
        .map(|r| ResourceInfo {
            name: r.name.clone(),
            uri_key: r.uri_key.clone(),
            label: r.name.clone(),
            plural_label: r.uri_key.clone(),
            group: r.group.clone(),
        })
        .collect();

    Ok(Json(ApiResponse::success(ResourceListResponse {
        resources,
    })))
}

/// Get resource index (paginated list)
pub async fn index_resource(
    State(_nova): State<Arc<Nova>>,
    Path(_resource): Path<String>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // This is a placeholder - actual implementation would query the database
    // based on the resource type

    Ok(Json(ApiResponse::success(serde_json::json!({
        "data": [],
        "meta": {
            "current_page": 1,
            "per_page": 15,
            "total": 0,
            "last_page": 1,
        }
    }))))
}

/// Show a single resource
pub async fn show_resource(
    State(_nova): State<Arc<Nova>>,
    Path((resource, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "resource": resource,
    }))))
}

/// Create a new resource
pub async fn create_resource(
    State(_nova): State<Arc<Nova>>,
    Path(_resource): Path<String>,
    Json(_data): Json<HashMap<String, Value>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": "new-id"}),
        "Resource created successfully",
    )))
}

/// Update an existing resource
pub async fn update_resource(
    State(_nova): State<Arc<Nova>>,
    Path((_resource, id)): Path<(String, String)>,
    Json(_data): Json<HashMap<String, Value>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "Resource updated successfully",
    )))
}

/// Delete a resource
pub async fn delete_resource(
    State(_nova): State<Arc<Nova>>,
    Path((_resource, id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "Resource deleted successfully",
    )))
}

/// Run an action on resources
pub async fn run_action(
    State(_nova): State<Arc<Nova>>,
    Path((_resource, _action)): Path<(String, String)>,
    Json(payload): Json<ActionPayload>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"affected": payload.resources.len()}),
        "Action executed successfully",
    )))
}

#[derive(Debug, Deserialize)]
pub struct ActionPayload {
    pub resources: Vec<String>,
    pub fields: HashMap<String, Value>,
}

/// Get filters for a resource
pub async fn get_filters(
    State(_nova): State<Arc<Nova>>,
    Path(_resource): Path<String>,
) -> Result<Json<ApiResponse<Vec<Value>>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(vec![])))
}

/// Get lens data for a resource
pub async fn get_lens(
    State(_nova): State<Arc<Nova>>,
    Path((_resource, _lens)): Path<(String, String)>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "data": [],
        "meta": {
            "current_page": 1,
            "per_page": 15,
            "total": 0,
        }
    }))))
}

/// Export resources
pub async fn export_resource(
    State(_nova): State<Arc<Nova>>,
    Path(_resource): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Placeholder implementation
    let content = match params.format.as_deref() {
        Some("csv") => "id,name\n1,Item 1\n2,Item 2",
        _ => r#"[{"id": 1, "name": "Item 1"}, {"id": 2, "name": "Item 2"}]"#,
    };

    Ok((
        StatusCode::OK,
        [
            ("Content-Type", "text/csv"),
            ("Content-Disposition", "attachment; filename=export.csv"),
        ],
        content,
    ))
}

#[derive(Debug, Deserialize)]
pub struct ExportParams {
    pub format: Option<String>,
}

/// List all dashboards
pub async fn list_dashboards(
    State(nova): State<Arc<Nova>>,
) -> Result<Json<ApiResponse<DashboardListResponse>>, StatusCode> {
    let dashboards: Vec<DashboardInfo> = nova
        .dashboards()
        .iter()
        .map(|d| DashboardInfo {
            name: d.name.clone(),
            uri_key: d.uri_key.clone(),
        })
        .collect();

    Ok(Json(ApiResponse::success(DashboardListResponse {
        dashboards,
    })))
}

/// Get a specific dashboard
pub async fn get_dashboard(
    State(_nova): State<Arc<Nova>>,
    Path(dashboard): Path<String>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "name": dashboard,
        "cards": [],
    }))))
}

/// Get value metric data
pub async fn get_value_metric(
    State(_nova): State<Arc<Nova>>,
    Path(_metric): Path<String>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "value": 1234,
        "prefix": "$",
        "previous": 1100,
        "increase": 134,
        "increase_percentage": 12.18,
    }))))
}

/// Get trend metric data
pub async fn get_trend_metric(
    State(_nova): State<Arc<Nova>>,
    Path(_metric): Path<String>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "data": {
            "2024-01": 100,
            "2024-02": 120,
            "2024-03": 150,
        },
        "trend": "up",
    }))))
}

/// Get partition metric data
pub async fn get_partition_metric(
    State(_nova): State<Arc<Nova>>,
    Path(_metric): Path<String>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "segments": [
            {"label": "Category A", "value": 45, "color": "#4299E1"},
            {"label": "Category B", "value": 30, "color": "#48BB78"},
            {"label": "Category C", "value": 25, "color": "#ECC94B"},
        ]
    }))))
}

/// Global search across all resources
pub async fn global_search(
    State(_nova): State<Arc<Nova>>,
    Query(_params): Query<SearchParams>,
) -> Result<Json<ApiResponse<Value>>, StatusCode> {
    // Placeholder implementation
    Ok(Json(ApiResponse::success(serde_json::json!({
        "results": []
    }))))
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}

/// Get Nova configuration
pub async fn get_config(
    State(nova): State<Arc<Nova>>,
) -> Result<Json<ApiResponse<ConfigResponse>>, StatusCode> {
    let config = nova.config();

    Ok(Json(ApiResponse::success(ConfigResponse {
        name: config.name.clone(),
        logo: config.logo.clone(),
        theme: config.theme.clone(),
        primary_color: config.primary_color.clone(),
        global_search: config.global_search,
        per_page_options: config.per_page_options.clone(),
        default_per_page: config.default_per_page,
    })))
}
