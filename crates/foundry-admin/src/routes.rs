//! HTTP route handlers for admin panel

use crate::admin::AdminPanel;
use crate::dashboard::DashboardData;
use crate::resource::ListQuery;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn dashboard(State(panel): State<Arc<AdminPanel>>) -> impl IntoResponse {
    match panel.dashboard().render().await {
        Ok(data) => {
            let html = panel
                .templates()
                .render_dashboard(&data)
                .unwrap_or_else(|e| format!("Template error: {}", e));
            Html(html)
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

pub async fn login(State(panel): State<Arc<AdminPanel>>) -> impl IntoResponse {
    let html = panel
        .templates()
        .render_login(None)
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html)
}

pub async fn do_login(
    State(panel): State<Arc<AdminPanel>>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    // TODO: Implement actual authentication
    // For now, just redirect to dashboard
    if form.email == "admin@example.com" && form.password == "password" {
        Redirect::to(&format!("{}/", panel.config().prefix))
    } else {
        let html = panel
            .templates()
            .render_login(Some("Invalid credentials".to_string()))
            .unwrap_or_else(|e| format!("Template error: {}", e));
        Redirect::to(&format!("{}/login", panel.config().prefix))
    }
}

pub async fn logout(State(panel): State<Arc<AdminPanel>>) -> impl IntoResponse {
    Redirect::to(&format!("{}/login", panel.config().prefix))
}

pub async fn list_resources(State(panel): State<Arc<AdminPanel>>) -> impl IntoResponse {
    let resources = panel.list_resources();
    let html = format!(
        r#"
        <html>
        <head><title>Resources</title></head>
        <body>
            <h1>Available Resources</h1>
            <ul>
            {}
            </ul>
        </body>
        </html>
        "#,
        resources
            .iter()
            .map(|r| format!(r#"<li><a href="/admin/resources/{}">{}</a></li>"#, r, r))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Html(html)
}

#[derive(Debug, Deserialize)]
pub struct ListParams {
    page: Option<usize>,
    per_page: Option<usize>,
    search: Option<String>,
}

pub async fn show_resource(
    State(panel): State<Arc<AdminPanel>>,
    Path(resource_name): Path<String>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    let query = ListQuery {
        page: params.page.unwrap_or(1),
        per_page: params.per_page.unwrap_or(25),
        search: params.search,
        filters: HashMap::new(),
        sort_by: None,
        sort_desc: false,
    };

    match resource.list(query).await {
        Ok(result) => {
            #[derive(Serialize)]
            struct Context {
                resource_name: String,
                fields: Vec<serde_json::Value>,
                data: Vec<serde_json::Value>,
                page: usize,
                per_page: usize,
                total: usize,
                total_pages: usize,
                search: Option<String>,
            }

            let ctx = Context {
                resource_name: resource_name.clone(),
                fields: resource
                    .config()
                    .fields
                    .iter()
                    .map(|f| json!({"name": f.name, "label": f.label}))
                    .collect(),
                data: result.data,
                page: result.page,
                per_page: result.per_page,
                total: result.total,
                total_pages: result.total_pages,
                search: params.search,
            };

            let html = panel
                .templates()
                .render("resource_list.html", &ctx)
                .unwrap_or_else(|e| format!("Template error: {}", e));
            Html(html).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn show(
    State(panel): State<Arc<AdminPanel>>,
    Path((resource_name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    match resource.get(&id).await {
        Ok(Some(data)) => Html(format!("<pre>{}</pre>", serde_json::to_string_pretty(&data).unwrap())).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Record not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn create_form(
    State(panel): State<Arc<AdminPanel>>,
    Path(resource_name): Path<String>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    #[derive(Serialize)]
    struct Context {
        title: String,
        resource_name: String,
        fields: Vec<serde_json::Value>,
        data: HashMap<String, String>,
        errors: Option<HashMap<String, Vec<String>>>,
    }

    let ctx = Context {
        title: format!("Create {}", resource_name),
        resource_name: resource_name.clone(),
        fields: serde_json::to_value(&resource.config().fields).unwrap().as_array().unwrap().clone(),
        data: HashMap::new(),
        errors: None,
    };

    let html = panel
        .templates()
        .render("resource_form.html", &ctx)
        .unwrap_or_else(|e| format!("Template error: {}", e));
    Html(html).into_response()
}

pub async fn store(
    State(panel): State<Arc<AdminPanel>>,
    Path(resource_name): Path<String>,
    Form(data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    let value = serde_json::to_value(&data).unwrap();
    match resource.create(value).await {
        Ok(_) => Redirect::to(&format!("{}/resources/{}", panel.config().prefix, resource_name)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn edit_form(
    State(panel): State<Arc<AdminPanel>>,
    Path((resource_name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    match resource.get(&id).await {
        Ok(Some(data)) => {
            #[derive(Serialize)]
            struct Context {
                title: String,
                resource_name: String,
                fields: Vec<serde_json::Value>,
                data: serde_json::Value,
                errors: Option<HashMap<String, Vec<String>>>,
            }

            let ctx = Context {
                title: format!("Edit {} #{}", resource_name, id),
                resource_name: resource_name.clone(),
                fields: serde_json::to_value(&resource.config().fields).unwrap().as_array().unwrap().clone(),
                data,
                errors: None,
            };

            let html = panel
                .templates()
                .render("resource_form.html", &ctx)
                .unwrap_or_else(|e| format!("Template error: {}", e));
            Html(html).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Record not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn update(
    State(panel): State<Arc<AdminPanel>>,
    Path((resource_name, id)): Path<(String, String)>,
    Form(data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    let value = serde_json::to_value(&data).unwrap();
    match resource.update(&id, value).await {
        Ok(_) => Redirect::to(&format!("{}/resources/{}", panel.config().prefix, resource_name)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn delete(
    State(panel): State<Arc<AdminPanel>>,
    Path((resource_name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let resource = match panel.get_resource(&resource_name) {
        Some(r) => r,
        None => return (StatusCode::NOT_FOUND, "Resource not found".to_string()).into_response(),
    };

    match resource.delete(&id).await {
        Ok(_) => Redirect::to(&format!("{}/resources/{}", panel.config().prefix, resource_name)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn users_index() -> impl IntoResponse {
    Html("<h1>User Management</h1><p>Coming soon...</p>")
}

pub async fn settings() -> impl IntoResponse {
    Html("<h1>Settings</h1><p>Coming soon...</p>")
}

pub async fn activity_log() -> impl IntoResponse {
    Html("<h1>Activity Log</h1><p>Coming soon...</p>")
}
