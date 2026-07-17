//! Inertia middleware for Axum
//!
//! Handles version checking, shared props injection, and response formatting.
//!
//! The middleware is the **required** companion to handlers that return [`crate::render::Inertia`].
//! It has access to the request context that [`IntoResponse`] does not, and uses it to:
//!
//! * Detect browser vs. XHR requests and render HTML or JSON accordingly.
//! * Merge shared props into every Inertia response.
//! * Inject the real request URI and configured asset version.
//! * Return 409 + `X-Inertia-Location` on asset-version mismatch.

use crate::{
    config::InertiaConfig,
    props::SharedProps,
    response::{InertiaResponse, PendingInertia},
};
use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tower::{Layer, Service};

/// Inertia middleware — used with [`axum::middleware::from_fn_with_state`].
///
/// For the idiomatic Tower-layer API use [`InertiaMiddlewareLayer`] instead.
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

    /// Create a Tower middleware layer (preferred API).
    pub fn layer(config: InertiaConfig) -> InertiaMiddlewareLayer {
        InertiaMiddlewareLayer::new(config)
    }

    /// Access shared props so you can add global data before the server starts.
    pub fn shared_props(&self) -> &SharedProps {
        &self.shared_props
    }

    /// Access the configuration.
    pub fn config(&self) -> &InertiaConfig {
        &self.config
    }

    /// Handle an incoming request (for use with `axum::middleware::from_fn`).
    pub async fn handle(&self, req: Request, next: Next) -> Response {
        let is_inertia_request = InertiaResponse::is_inertia_request(req.headers());
        let req_uri = req.uri().to_string();

        // Version-mismatch check: only for Inertia XHR requests.
        if is_inertia_request {
            if let Some(version) = req.headers().get("X-Inertia-Version") {
                if let Ok(req_version) = version.to_str() {
                    let current_version = self.config.get_version();
                    if req_version != current_version {
                        // Inertia.js protocol: 409 + X-Inertia-Location triggers a hard reload.
                        // The client IGNORES the standard `Location` header on 409 responses.
                        return (
                            StatusCode::CONFLICT,
                            [("X-Inertia-Location", req_uri)],
                        )
                            .into_response();
                    }
                }
            }
        }

        let response = next.run(req).await;

        // If the handler returned an `Inertia` value, finalize the response here
        // where we have request context.
        if let Some(pending) = response
            .extensions()
            .get::<PendingInertia>()
            .cloned()
        {
            return self.finalize_inertia(pending, is_inertia_request, req_uri).await;
        }

        // Non-Inertia response: pass through without modification.
        response
    }

    async fn finalize_inertia(
        &self,
        pending: PendingInertia,
        is_inertia_request: bool,
        req_uri: String,
    ) -> Response {
        // Shared props are the base; handler props override on collision.
        let shared = self.shared_props.all().await;
        let merged_props = shared.merge(pending.inertia_response.props.clone());

        let url = pending.explicit_url.unwrap_or(req_uri);
        let version = self.config.get_version();

        let final_inertia = InertiaResponse {
            props: merged_props,
            url,
            version,
            ..pending.inertia_response
        };

        if is_inertia_request {
            let mut r = final_inertia.into_json_response();
            r.headers_mut()
                .insert("X-Inertia", HeaderValue::from_static("true"));
            r
        } else {
            final_inertia
                .into_html_response(&self.config.root_view)
                .unwrap_or_else(|e| {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                })
        }
    }
}

/// Tower layer for the Inertia middleware.
///
/// # Example
///
/// ```rust,ignore
/// let layer = InertiaMiddlewareLayer::new(config);
/// layer.shared_props().add("user", current_user).await;
///
/// let app = Router::new()
///     .route("/", get(index))
///     .layer(layer);
/// ```
#[derive(Clone)]
pub struct InertiaMiddlewareLayer {
    config: Arc<InertiaConfig>,
    shared_props: Arc<SharedProps>,
}

impl InertiaMiddlewareLayer {
    /// Create a new layer.
    pub fn new(config: InertiaConfig) -> Self {
        Self {
            config: Arc::new(config),
            shared_props: Arc::new(SharedProps::new()),
        }
    }

    /// Access shared props to add global data before the server starts.
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

/// Inertia middleware service (produced by [`InertiaMiddlewareLayer`]).
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
        let shared_props = self.shared_props.clone();

        Box::pin(async move {
            // Capture request metadata before the request is consumed.
            let is_inertia_request = InertiaResponse::is_inertia_request(req.headers());
            let req_uri = req.uri().to_string();

            // Version-mismatch check: only for Inertia XHR requests.
            if is_inertia_request {
                if let Some(version) = req.headers().get("X-Inertia-Version") {
                    if let Ok(req_version) = version.to_str() {
                        let current_version = config.get_version();
                        if req_version != current_version {
                            // Inertia.js protocol: 409 + X-Inertia-Location triggers a hard reload.
                            // The client IGNORES the standard `Location` header on 409 responses.
                            return Ok((
                                StatusCode::CONFLICT,
                                [("X-Inertia-Location", req_uri)],
                            )
                                .into_response());
                        }
                    }
                }
            }

            let response = inner.call(req).await?;

            // If the handler returned an `Inertia` value, it placed a `PendingInertia`
            // marker in the response extensions.  Finalize the response here where we
            // have the request context (XHR vs browser, URL, config version, shared props).
            if let Some(pending) = response.extensions().get::<PendingInertia>().cloned() {
                // Shared props are the base; handler props override on collision.
                let shared = shared_props.all().await;
                let merged_props = shared.merge(pending.inertia_response.props.clone());

                let url = pending.explicit_url.unwrap_or(req_uri);
                let version = config.get_version();

                let final_inertia = InertiaResponse {
                    props: merged_props,
                    url,
                    version,
                    ..pending.inertia_response
                };

                return Ok(if is_inertia_request {
                    // XHR: JSON payload + X-Inertia: true response header.
                    let mut r = final_inertia.into_json_response();
                    r.headers_mut()
                        .insert("X-Inertia", HeaderValue::from_static("true"));
                    r
                } else {
                    // Browser: full HTML page with embedded JSON.
                    final_inertia
                        .into_html_response(&config.root_view)
                        .unwrap_or_else(|e| {
                            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                        })
                });
            }

            // Non-Inertia response: pass through without modification.
            // Do NOT add X-Inertia headers to responses that are not Inertia pages.
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
        let _layer = InertiaMiddlewareLayer::new(config);
        // Just verify it can be created without panicking
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
