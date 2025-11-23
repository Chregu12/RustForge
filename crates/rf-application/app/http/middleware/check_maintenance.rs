use axum::{
    body::Body,
    http::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Json},
};
use serde_json::{json, Value};
use std::fs;

const MAINTENANCE_FILE: &str = ".foundry/down";

pub async fn check_maintenance(req: Request<Body>, next: Next) -> impl IntoResponse {
    if fs::metadata(MAINTENANCE_FILE).is_ok() {
        let content = fs::read_to_string(MAINTENANCE_FILE).unwrap_or_else(|_|
            "{\"message\": \"Application is in maintenance mode.\"}".to_string(),
        );
        let data: Value = serde_json::from_str(&content).unwrap_or_else(|_|
            json!({
                "message": "Application is in maintenance mode."
            })
        );

        return (StatusCode::SERVICE_UNAVAILABLE, Json(data)).into_response();
    }

    next.run(req).await.into_response()
}
