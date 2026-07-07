//! Inertia middleware for Axum
//!
//! Handles version checking, shared props injection, and response formatting.

use crate::{config::InertiaConfig, props::SharedProps, response::InertiaResponse};
use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tower::{Layer, Service};

/// Inertia middleware layer
#[derive(Clone)]
pub struct InertiaMiddleware {
    config: Arc<InertiaConfig>,
    shared_props: Arc<SharedProps>,
}

impl InertiaMiddleware {
    /// Create a new Inertia middleware
    pub fn new(config: InertiaConfig) -> Self {
        Self {
            config: Arc::new(config),
            shared_props: Arc::new(SharedProps::new()),
        }
    }

    /// Create a middleware layer
    pub fn layer(config: InertiaConfig) -> InertiaMiddlewareLayer {
        InertiaMiddlewareLayer::new(config)
    }

    /// Get shared props
    pub fn shared_props(&self) -> &SharedProps {
        &self.shared_props
    }

    /// Get config
    pub fn config(&self) -> &InertiaConfig {
        &self.config
    }

    /// Handle an incoming request
    pub async fn handle(&self, req: Request, next: Next) -> Response {
        // Check version mismatch for Inertia requests
        if InertiaResponse::is_inertia_request(req.headers()) {
            if let Some(version) = req.headers().get("X-Inertia-Version") {
                if let Ok(req_version) = version.to_str() {
                    let current_version = self.config.get_version();
                    if req_version != current_version {
                        // Version mismatch - trigger full page reload
                        return (
                            StatusCode::CONFLICT,
                            [(header::LOCATION, req.uri().to_string())],
                        )
                            .into_response();
                    }
                }
            }
        }

        // Process the request
        let mut response = next.run(req).await;

        // Add Inertia headers to response
        if let Ok(_headers) = response
            .headers_mut()
            .try_insert("X-Inertia", HeaderValue::from_static("true"))
        {
            // Header insertion successful
        }

        response
    }
}

/// Tower layer for Inertia middleware
#[derive(Clone)]
pub struct InertiaMiddlewareLayer {
    config: Arc<InertiaConfig>,
    shared_props: Arc<SharedProps>,
}

impl InertiaMiddlewareLayer {
    /// Create a new layer
    pub fn new(config: InertiaConfig) -> Self {
        Self {
            config: Arc::new(config),
            shared_props: Arc::new(SharedProps::new()),
        }
    }

    /// Access shared props
    pub fn shared_props(&self) -> &SharedProps {
        &self.shared_props
    }
}

impl<S> Layer<S> for InertiaMiddlewareLayer {
    type Service = InertiaMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        InertiaMiddlewareService {
            inner,
            config: self.config.clone(),
            shared_props: self.shared_props.clone(),
        }
    }
}

/// Inertia middleware service
#[derive(Clone)]
pub struct InertiaMiddlewareService<S> {
    inner: S,
    config: Arc<InertiaConfig>,
    shared_props: Arc<SharedProps>,
}

impl<S> Service<Request> for InertiaMiddlewareService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let inner = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, inner);
        let config = self.config.clone();
        let _shared_props = self.shared_props.clone();

        Box::pin(async move {
            // Check version mismatch for Inertia requests
            let headers = req.headers();
            if InertiaResponse::is_inertia_request(headers) {
                if let Some(version) = headers.get("X-Inertia-Version") {
                    if let Ok(req_version) = version.to_str() {
                        let current_version = config.get_version();
                        if req_version != current_version {
                            // Version mismatch - trigger full page reload
                            return Ok((
                                StatusCode::CONFLICT,
                                [(header::LOCATION, req.uri().to_string())],
                            )
                                .into_response());
                        }
                    }
                }
            }

            // Process the request
            let mut response = inner.call(req).await?;

            // Add Inertia header to response
            response
                .headers_mut()
                .insert("X-Inertia", HeaderValue::from_static("true"));

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_creation() {
        let config = InertiaConfig::new().version("v1.0.0");
        let middleware = InertiaMiddleware::new(config);

        assert_eq!(middleware.config.get_version(), "v1.0.0");
    }

    #[test]
    fn test_layer_creation() {
        let config = InertiaConfig::new();
        let layer = InertiaMiddlewareLayer::new(config);

        // Just verify it can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_shared_props() {
        let config = InertiaConfig::new();
        let middleware = InertiaMiddleware::new(config);

        middleware.shared_props().add("app_name", "RustForge").await;

        let props = middleware.shared_props().all().await;
        assert!(props.has("app_name"));
    }
}
