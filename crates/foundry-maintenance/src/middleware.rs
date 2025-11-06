//! Axum middleware for maintenance mode

use crate::config::MaintenanceState;
use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::path::PathBuf;

/// Maintenance mode middleware
#[derive(Clone)]
pub struct MaintenanceMiddleware {
    file_path: PathBuf,
}

impl MaintenanceMiddleware {
    /// Create new maintenance middleware
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    /// Check maintenance mode for a request
    pub async fn check(&self, headers: &HeaderMap, request: &Request) -> Option<Response> {
        if !self.file_path.exists() {
            return None;
        }

        // Read maintenance state
        let state = match self.read_state() {
            Ok(state) => state,
            Err(_) => return None,
        };

        // Check for bypass secret in headers
        if let Some(_secret) = &state.secret {
            if let Some(auth) = headers.get("X-Maintenance-Secret") {
                if let Ok(provided) = auth.to_str() {
                    if state.verify_secret(provided) {
                        return None; // Allow through
                    }
                }
            }

            // Check for bypass secret in query params
            if let Some(query) = request.uri().query() {
                for param in query.split('&') {
                    if let Some((key, value)) = param.split_once('=') {
                        if key == "secret" && state.verify_secret(value) {
                            return None; // Allow through
                        }
                    }
                }
            }
        }

        // Return 503 response
        Some(self.maintenance_response(&state))
    }

    /// Read maintenance state from file
    fn read_state(&self) -> Result<MaintenanceState, std::io::Error> {
        let content = std::fs::read_to_string(&self.file_path)?;
        serde_json::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Create maintenance response
    fn maintenance_response(&self, state: &MaintenanceState) -> Response {
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Maintenance Mode</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            margin: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
        }}
        .container {{
            text-align: center;
            padding: 2rem;
            max-width: 600px;
        }}
        h1 {{
            font-size: 3rem;
            margin-bottom: 1rem;
        }}
        p {{
            font-size: 1.25rem;
            opacity: 0.9;
        }}
        .time {{
            margin-top: 2rem;
            font-size: 0.875rem;
            opacity: 0.7;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔧 Maintenance Mode</h1>
        <p>{}</p>
        <div class="time">Enabled at: {}</div>
    </div>
</body>
</html>"#,
            state.display_message(),
            state.enabled_at
        );

        let mut response = (StatusCode::SERVICE_UNAVAILABLE, body).into_response();

        // Add Retry-After header if specified
        if let Some(retry_after) = state.retry_after {
            response.headers_mut().insert(
                "Retry-After",
                retry_after.to_string().parse().unwrap(),
            );
        }

        response
    }
}

/// Axum middleware function
pub async fn maintenance_check(
    middleware: MaintenanceMiddleware,
    request: Request,
    next: Next,
) -> Response {
    if let Some(response) = middleware.check(request.headers(), &request).await {
        return response;
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_middleware_new() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        let middleware = MaintenanceMiddleware::new(file_path.clone());
        assert_eq!(middleware.file_path, file_path);
    }

    #[test]
    fn test_read_state() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        let state = MaintenanceState::new(
            Some("Test".to_string()),
            Some("secret".to_string()),
        );

        std::fs::write(&file_path, serde_json::to_string(&state).unwrap()).unwrap();

        let middleware = MaintenanceMiddleware::new(file_path);
        let read_state = middleware.read_state().unwrap();

        assert_eq!(read_state.message, state.message);
    }

    #[test]
    fn test_maintenance_response() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        let middleware = MaintenanceMiddleware::new(file_path);
        let state = MaintenanceState::new(
            Some("Custom message".to_string()),
            None,
        );

        let response = middleware.maintenance_response(&state);
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_maintenance_response_with_retry() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".maintenance");

        let middleware = MaintenanceMiddleware::new(file_path);
        let state = MaintenanceState::new(None, None).with_retry_after(3600);

        let response = middleware.maintenance_response(&state);
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key("Retry-After"));
    }
}
