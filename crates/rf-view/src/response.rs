use crate::view::View;
use axum::response::{Html, IntoResponse, Response};
use http::StatusCode;

/// An HTTP response containing a rendered view
///
/// This integrates views with Axum's response system.
///
/// # Example
///
/// ```rust,no_run
/// use rf_view::View;
/// use serde_json::json;
/// use axum::response::IntoResponse;
///
/// async fn index() -> impl IntoResponse {
///     View::make("home", json!({
///         "title": "Home",
///         "user": "John Doe"
///     }))
/// }
/// ```
pub struct ViewResponse {
    pub(crate) view: View,
}

impl IntoResponse for ViewResponse {
    fn into_response(self) -> Response {
        // Render the view
        let html = match tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.view.render())
        }) {
            Ok(html) => html,
            Err(e) => {
                tracing::error!("View rendering failed: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!(
                        "<h1>500 Internal Server Error</h1><p>Template rendering failed: {}</p>",
                        e
                    )),
                )
                    .into_response();
            }
        };

        Html(html).into_response()
    }
}

impl IntoResponse for View {
    fn into_response(self) -> Response {
        self.into_response().into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_response_creation() {
        // This test just checks that the API compiles
        // Real tests would need Axum runtime and templates
    }
}
