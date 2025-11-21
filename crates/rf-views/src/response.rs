use crate::{context::Context, engine::ViewEngine, error::ViewError};
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;

/// A view response that can be returned from Axum handlers
pub struct ViewResponse {
    template: String,
    context: Context,
    status: StatusCode,
    engine: Arc<ViewEngine>,
}

impl ViewResponse {
    /// Create a new view response
    pub fn new(engine: Arc<ViewEngine>, template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            context: Context::new(),
            status: StatusCode::OK,
            engine,
        }
    }

    /// Create a view response with data
    pub fn with_data<T: Serialize>(
        engine: Arc<ViewEngine>,
        template: impl Into<String>,
        data: T,
    ) -> Result<Self, ViewError> {
        let context = Context::from_value(data)?;
        Ok(Self {
            template: template.into(),
            context,
            status: StatusCode::OK,
            engine,
        })
    }

    /// Create a view response with context
    pub fn with_context(
        engine: Arc<ViewEngine>,
        template: impl Into<String>,
        context: Context,
    ) -> Self {
        Self {
            template: template.into(),
            context,
            status: StatusCode::OK,
            engine,
        }
    }

    /// Set the HTTP status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Add a value to the context
    pub fn with<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Merge another context
    pub fn merge_context(mut self, context: Context) -> Self {
        self.context.merge(context);
        self
    }
}

impl IntoResponse for ViewResponse {
    fn into_response(self) -> Response {
        match self.engine.render(&self.template, &self.context) {
            Ok(html) => (self.status, Html(html)).into_response(),
            Err(e) => {
                eprintln!("View rendering error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error rendering template: {}", e),
                )
                    .into_response()
            }
        }
    }
}

/// Helper function to create a view response
pub fn view(
    engine: Arc<ViewEngine>,
    template: impl Into<String>,
    data: impl Serialize,
) -> Result<ViewResponse, ViewError> {
    ViewResponse::with_data(engine, template, data)
}

/// Helper function to create a view response with context
pub fn view_context(
    engine: Arc<ViewEngine>,
    template: impl Into<String>,
    context: Context,
) -> ViewResponse {
    ViewResponse::with_context(engine, template, context)
}

/// Macro for creating view responses
#[macro_export]
macro_rules! view {
    ($engine:expr, $template:expr) => {
        $crate::response::ViewResponse::new($engine, $template)
    };
    ($engine:expr, $template:expr, $data:expr) => {
        $crate::response::ViewResponse::with_data($engine, $template, $data)
    };
}

/// Builder for creating HTML responses with additional headers
pub struct HtmlResponse {
    html: String,
    status: StatusCode,
}

impl HtmlResponse {
    /// Create a new HTML response
    pub fn new(html: impl Into<String>) -> Self {
        Self {
            html: html.into(),
            status: StatusCode::OK,
        }
    }

    /// Set the status code
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

impl IntoResponse for HtmlResponse {
    fn into_response(self) -> Response {
        (self.status, Html(self.html)).into_response()
    }
}

/// Render a template directly to an HTML response
pub fn render(
    engine: &ViewEngine,
    template: impl AsRef<str>,
    data: impl Serialize,
) -> Result<Html<String>, ViewError> {
    let html = engine.render_with_data(template.as_ref(), data)?;
    Ok(Html(html))
}

/// Render a template with context directly to an HTML response
pub fn render_context(
    engine: &ViewEngine,
    template: impl AsRef<str>,
    context: &Context,
) -> Result<Html<String>, ViewError> {
    let html = engine.render(template.as_ref(), context)?;
    Ok(Html(html))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_engine() -> (Arc<ViewEngine>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let views_path = temp_dir.path().join("views");
        fs::create_dir_all(&views_path).unwrap();

        let template_path = views_path.join("test.tera");
        fs::write(&template_path, "Hello {{ name }}!").unwrap();

        let engine = Arc::new(ViewEngine::new(views_path.to_str().unwrap()).unwrap());
        (engine, temp_dir)
    }

    #[test]
    fn test_view_response_creation() {
        let (engine, _temp_dir) = create_test_engine();

        let response = ViewResponse::new(engine.clone(), "test")
            .with("name", "World");

        // Convert to response to trigger rendering
        let axum_response = response.into_response();
        assert_eq!(axum_response.status(), StatusCode::OK);
    }

    #[test]
    fn test_view_response_with_data() {
        let (engine, _temp_dir) = create_test_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "Rust".to_string(),
        };

        let response = ViewResponse::with_data(engine.clone(), "test", &data).unwrap();
        let axum_response = response.into_response();
        assert_eq!(axum_response.status(), StatusCode::OK);
    }

    #[test]
    fn test_view_response_status() {
        let (engine, _temp_dir) = create_test_engine();

        let response = ViewResponse::new(engine.clone(), "test")
            .with("name", "World")
            .status(StatusCode::CREATED);

        let axum_response = response.into_response();
        assert_eq!(axum_response.status(), StatusCode::CREATED);
    }

    #[test]
    fn test_render_helper() {
        let (engine, _temp_dir) = create_test_engine();

        #[derive(Serialize)]
        struct Data {
            name: String,
        }

        let data = Data {
            name: "Helper".to_string(),
        };

        let html = render(&engine, "test", &data).unwrap();
        assert_eq!(html.0, "Hello Helper!");
    }

    #[test]
    fn test_html_response() {
        let response = HtmlResponse::new("<h1>Test</h1>")
            .status(StatusCode::OK);

        let axum_response = response.into_response();
        assert_eq!(axum_response.status(), StatusCode::OK);
    }
}
