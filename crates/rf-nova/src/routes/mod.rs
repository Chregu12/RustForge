pub mod api;
pub mod handlers;

use crate::Nova;
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;

/// Build Nova routes
pub fn build_routes(nova: Arc<Nova>) -> Router {
    let base_path = &nova.config().path;

    Router::new()
        // API routes
        .route(&format!("{}/api/resources", base_path), get(handlers::list_resources))
        .route(&format!("{}/api/resources/:resource", base_path), get(handlers::index_resource))
        .route(&format!("{}/api/resources/:resource/:id", base_path), get(handlers::show_resource))
        .route(&format!("{}/api/resources/:resource", base_path), post(handlers::create_resource))
        .route(&format!("{}/api/resources/:resource/:id", base_path), put(handlers::update_resource))
        .route(&format!("{}/api/resources/:resource/:id", base_path), delete(handlers::delete_resource))

        // Action routes
        .route(&format!("{}/api/resources/:resource/actions/:action", base_path), post(handlers::run_action))

        // Filter routes
        .route(&format!("{}/api/resources/:resource/filters", base_path), get(handlers::get_filters))

        // Lens routes
        .route(&format!("{}/api/resources/:resource/lenses/:lens", base_path), get(handlers::get_lens))

        // Export routes
        .route(&format!("{}/api/resources/:resource/export", base_path), get(handlers::export_resource))

        // Dashboard routes
        .route(&format!("{}/api/dashboards", base_path), get(handlers::list_dashboards))
        .route(&format!("{}/api/dashboards/:dashboard", base_path), get(handlers::get_dashboard))

        // Metric routes
        .route(&format!("{}/api/metrics/value/:metric", base_path), get(handlers::get_value_metric))
        .route(&format!("{}/api/metrics/trend/:metric", base_path), get(handlers::get_trend_metric))
        .route(&format!("{}/api/metrics/partition/:metric", base_path), get(handlers::get_partition_metric))

        // Search routes
        .route(&format!("{}/api/search", base_path), get(handlers::global_search))

        // Configuration routes
        .route(&format!("{}/api/config", base_path), get(handlers::get_config))

        // Attach Nova instance as state
        .with_state(nova)
}
