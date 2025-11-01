//! Middleware support for request/response processing

use async_trait::async_trait;

/// Middleware trait
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder>;

    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response>;
}

/// Chain of middlewares
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
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging middleware example
pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn process_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        tracing::info!("Sending HTTP request");
        Ok(request)
    }

    async fn process_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        tracing::info!("Received HTTP response: {}", response.status());
        Ok(response)
    }
}
