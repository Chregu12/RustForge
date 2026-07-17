//! Middleware support for request/response processing
//!
//! Provides lifecycle hooks for HTTP requests:
//! - `process_request`: Modify requests before sending
//! - `process_response`: Inspect/modify responses after receiving
//! - `on_error`: Handle errors during request execution

use async_trait::async_trait;

/// Middleware trait for request/response lifecycle hooks
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process request before sending (beforeRequest hook)
    async fn process_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder>;

    /// Process response after receiving (afterResponse hook - Laravel 12 feature)
    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response>;

    /// Handle errors during request execution
    async fn on_error(&self, _error: &anyhow::Error) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Chain of middlewares applied in order
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add(&mut self, middleware: Box<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub async fn process_request(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        for middleware in &self.middlewares {
            request = middleware.process_request(request).await?;
        }
        Ok(request)
    }

    pub async fn process_response(
        &self,
        mut response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        for middleware in &self.middlewares {
            response = middleware.process_response(response).await?;
        }
        Ok(response)
    }

    pub async fn on_error(&self, error: &anyhow::Error) -> anyhow::Result<()> {
        for middleware in &self.middlewares {
            middleware.on_error(error).await?;
        }
        Ok(())
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging middleware - logs requests and responses
pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        tracing::info!("HTTP request outgoing");
        Ok(request)
    }

    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        tracing::info!(
            status = %response.status(),
            url = %response.url(),
            "HTTP response received"
        );
        Ok(response)
    }

    async fn on_error(&self, error: &anyhow::Error) -> anyhow::Result<()> {
        tracing::error!(error = %error, "HTTP request failed");
        Ok(())
    }
}

/// Header injection middleware - adds default headers to every request
pub struct HeaderMiddleware {
    headers: Vec<(String, String)>,
}

impl HeaderMiddleware {
    pub fn new(headers: Vec<(String, String)>) -> Self {
        Self { headers }
    }
}

#[async_trait]
impl Middleware for HeaderMiddleware {
    async fn process_request(
        &self,
        mut request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        for (key, value) in &self.headers {
            request = request.header(key.as_str(), value.as_str());
        }
        Ok(request)
    }

    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

/// Callback-based afterResponse hook (Laravel 12 style)
///
/// # Example
///
/// ```rust,ignore
/// use rf_http_client::middleware::AfterResponseHook;
///
/// let hook = AfterResponseHook::new(|resp| {
///     println!("Got status: {}", resp.status());
///     Ok(resp)
/// });
/// ```
pub struct AfterResponseHook<F>
where
    F: Fn(reqwest::Response) -> anyhow::Result<reqwest::Response> + Send + Sync,
{
    callback: F,
}

impl<F> AfterResponseHook<F>
where
    F: Fn(reqwest::Response) -> anyhow::Result<reqwest::Response> + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

#[async_trait]
impl<F> Middleware for AfterResponseHook<F>
where
    F: Fn(reqwest::Response) -> anyhow::Result<reqwest::Response> + Send + Sync,
{
    async fn process_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        Ok(request)
    }

    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        (self.callback)(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_chain_creation() {
        let chain = MiddlewareChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_middleware_chain_add() {
        let mut chain = MiddlewareChain::new();
        chain.add(Box::new(LoggingMiddleware));
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_header_middleware_creation() {
        let headers = vec![
            ("X-Api-Key".to_string(), "secret".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];
        let _middleware = HeaderMiddleware::new(headers);
    }
}
