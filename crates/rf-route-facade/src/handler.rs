//! Handler trait and implementations for route handlers.
//!
//! This module provides a flexible handler system that supports closures,
//! async functions, and controller methods.

use axum::{
    response::IntoResponse,
    extract::Request,
};
use std::future::Future;
use std::pin::Pin;

/// Type alias for a boxed future that returns a response
pub type BoxedFuture = Pin<Box<dyn Future<Output = axum::response::Response> + Send>>;

/// Handler trait for route handlers.
///
/// This trait is implemented for closures and functions that can handle HTTP requests.
pub trait Handler: Clone + Send + Sync + 'static {
    /// Handle the request and return a response.
    fn call(&self, req: Request) -> BoxedFuture;
}

/// A function-based handler that wraps a closure.
#[derive(Clone)]
pub struct HandlerFunc<F> {
    func: F,
}

impl<F> HandlerFunc<F> {
    /// Create a new handler from a function.
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F, Fut, Res> Handler for HandlerFunc<F>
where
    F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + 'static,
{
    fn call(&self, req: Request) -> BoxedFuture {
        let func = self.func.clone();
        Box::pin(async move {
            let result = func(req).await;
            result.into_response()
        })
    }
}

/// Helper function to create a handler from a closure.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_route_facade::handler::handler_fn;
///
/// let h = handler_fn(|_req| async {
///     "Hello, World!"
/// });
/// ```
pub fn handler_fn<F, Fut, Res>(func: F) -> HandlerFunc<F>
where
    F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + 'static,
{
    HandlerFunc::new(func)
}

/// Trait for converting values into handlers.
pub trait IntoHandler {
    /// The handler type that this converts into.
    type Handler: Handler;

    /// Convert this value into a handler.
    fn into_handler(self) -> Self::Handler;
}

impl<F, Fut, Res> IntoHandler for F
where
    F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse + 'static,
{
    type Handler = HandlerFunc<F>;

    fn into_handler(self) -> Self::Handler {
        handler_fn(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::StatusCode;

    #[tokio::test]
    async fn test_handler_func() {
        let handler = handler_fn(|_req: Request| async {
            (StatusCode::OK, "Hello, World!")
        });

        let req = Request::new(Body::empty());
        let response = handler.call(req).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_into_handler() {
        let func = |_req: Request| async {
            "Test response"
        };

        let handler = func.into_handler();
        let req = Request::new(Body::empty());
        let response = handler.call(req).await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
