use crate::{context::Context, engine::ViewEngine, error::ViewResult};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Serialize;
use std::sync::Arc;

/// Render a view and return an HTML response
pub fn view(
    engine: &ViewEngine,
    template: impl AsRef<str>,
    data: impl Serialize,
) -> ViewResult<Html<String>> {
    let html = engine.render_with_data(template.as_ref(), data)?;
    Ok(Html(html))
}

/// Render a view with context and return an HTML response
pub fn view_with_context(
    engine: &ViewEngine,
    template: impl AsRef<str>,
    context: &Context,
) -> ViewResult<Html<String>> {
    let html = engine.render(template.as_ref(), context)?;
    Ok(Html(html))
}

/// Create a redirect response
pub fn redirect(path: impl AsRef<str>) -> Redirect {
    Redirect::to(path.as_ref())
}

/// Create a redirect response with a success flash message
pub fn redirect_with_success(
    engine: &ViewEngine,
    path: impl AsRef<str>,
    message: impl Into<String>,
) -> Redirect {
    engine.set_flash("success", message);
    Redirect::to(path.as_ref())
}

/// Create a redirect response with an error flash message
pub fn redirect_with_error(
    engine: &ViewEngine,
    path: impl AsRef<str>,
    message: impl Into<String>,
) -> Redirect {
    engine.set_flash("error", message);
    Redirect::to(path.as_ref())
}

/// Create a redirect response with an info flash message
pub fn redirect_with_info(
    engine: &ViewEngine,
    path: impl AsRef<str>,
    message: impl Into<String>,
) -> Redirect {
    engine.set_flash("info", message);
    Redirect::to(path.as_ref())
}

/// Create a redirect response with a warning flash message
pub fn redirect_with_warning(
    engine: &ViewEngine,
    path: impl AsRef<str>,
    message: impl Into<String>,
) -> Redirect {
    engine.set_flash("warning", message);
    Redirect::to(path.as_ref())
}

/// Create a redirect back response (typically used after form submissions)
pub fn redirect_back(referer: Option<&str>) -> Redirect {
    Redirect::to(referer.unwrap_or("/"))
}

/// Create a JSON error response
pub fn json_error(message: impl Into<String>, status: StatusCode) -> Response {
    let json = serde_json::json!({
        "error": message.into(),
    });

    (status, axum::Json(json)).into_response()
}

/// Create a JSON success response
pub fn json_success<T: Serialize>(data: T) -> Response {
    (StatusCode::OK, axum::Json(data)).into_response()
}

/// Share data with all views (useful for middleware)
pub struct ViewShare {
    engine: Arc<ViewEngine>,
}

impl ViewShare {
    /// Create a new view share instance
    pub fn new(engine: Arc<ViewEngine>) -> Self {
        Self { engine }
    }

    /// Share a value with all views
    pub fn share<T: Serialize>(&self, key: impl Into<String>, value: T) {
        // This would typically be implemented with a global context
        // that gets merged with every render call
        // For now, this is a placeholder for the concept
        drop((key, value));
    }

    /// Get the engine
    pub fn engine(&self) -> &ViewEngine {
        &self.engine
    }
}

/// Builder for creating view responses with chained methods
pub struct ViewBuilder {
    engine: Arc<ViewEngine>,
    template: String,
    context: Context,
    status: StatusCode,
}

impl ViewBuilder {
    /// Create a new view builder
    pub fn new(engine: Arc<ViewEngine>, template: impl Into<String>) -> Self {
        Self {
            engine,
            template: template.into(),
            context: Context::new(),
            status: StatusCode::OK,
        }
    }

    /// Add data to the view
    pub fn with<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Merge context into the view
    pub fn merge(mut self, context: Context) -> Self {
        self.context.merge(context);
        self
    }

    /// Set the status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Render the view
    pub fn render(self) -> ViewResult<Html<String>> {
        let html = self.engine.render(&self.template, &self.context)?;
        Ok(Html(html))
    }

    /// Convert to a response
    pub fn into_response(self) -> Response {
        let status = self.status;
        match self.render() {
            Ok(html) => (status, html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("View error: {}", e),
            )
                .into_response(),
        }
    }
}

/// Macro for quickly creating views
#[macro_export]
macro_rules! render_view {
    ($engine:expr, $template:expr) => {
        $crate::helpers::view($engine, $template, &())
    };
    ($engine:expr, $template:expr, $data:expr) => {
        $crate::helpers::view($engine, $template, $data)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_engine() -> (ViewEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let views_path = temp_dir.path().join("views");
        fs::create_dir_all(&views_path).unwrap();

        let template_path = views_path.join("test.tera");
        fs::write(&template_path, "Hello {{ name }}!").unwrap();

        let engine = ViewEngine::new(views_path.to_str().unwrap()).unwrap();
        (engine, temp_dir)
    }

    #[test]
    fn test_view_helper() {
        let (engine, _temp_dir) = create_test_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "World".to_string(),
        };

        let result = view(&engine, "test", &data).unwrap();
        assert_eq!(result.0, "Hello World!");
    }

    #[test]
    fn test_redirect_with_success() {
        let (engine, _temp_dir) = create_test_engine();

        let _redirect = redirect_with_success(&engine, "/posts", "Post created successfully!");

        // Verify flash was set (in a real scenario, you'd check the session)
        // For now, we just verify it doesn't panic
    }

    #[test]
    fn test_view_builder() {
        let (engine, _temp_dir) = create_test_engine();

        let result = ViewBuilder::new(Arc::new(engine), "test")
            .with("name", "Builder")
            .render()
            .unwrap();

        assert_eq!(result.0, "Hello Builder!");
    }

    #[test]
    fn test_view_builder_status() {
        let (engine, _temp_dir) = create_test_engine();

        let builder = ViewBuilder::new(Arc::new(engine), "test")
            .with("name", "Status")
            .status(StatusCode::CREATED);

        let response = builder.into_response();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[test]
    fn test_json_error() {
        let response = json_error("Something went wrong", StatusCode::BAD_REQUEST);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_json_success() {
        #[derive(Serialize)]
        struct SuccessData {
            message: String,
        }

        let data = SuccessData {
            message: "Success!".to_string(),
        };

        let response = json_success(&data);
        assert_eq!(response.status(), StatusCode::OK);
    }
}
