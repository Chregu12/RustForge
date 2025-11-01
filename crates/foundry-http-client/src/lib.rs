//! Foundry HTTP Client - Guzzle-style HTTP Client
//!
//! Provides a fluent request builder with:
//! - GET, POST, PUT, PATCH, DELETE methods
//! - JSON & Form data support
//! - Headers & Authentication (Basic, Bearer, Custom)
//! - Timeout & Retry logic
//! - Response parsing
//! - Certificate validation
//! - Middleware support
//!
//! # Example
//!
//! ```no_run
//! use foundry_http_client::HttpClient;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let client = HttpClient::new();
//!
//!     let response = client
//!         .get("https://api.example.com/users")
//!         .header("Accept", "application/json")
//!         .bearer_auth("token123")
//!         .timeout(30)
//!         .send()
//!         .await?;
//!
//!     let users: Vec<User> = response.json().await?;
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod middleware;
pub mod request;
pub mod response;
pub mod retry;

pub use auth::{Auth, AuthType};
pub use middleware::{Middleware, MiddlewareChain};
pub use request::{RequestBuilder, RequestMethod};
pub use response::Response;
pub use retry::{RetryConfig, RetryPolicy};

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

/// Main HTTP client
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    middlewares: Arc<MiddlewareChain>,
    retry_config: RetryConfig,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            middlewares: Arc::new(MiddlewareChain::new()),
            retry_config: RetryConfig::default(),
        }
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    pub fn get(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Get, url)
    }

    pub fn post(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Post, url)
    }

    pub fn put(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Put, url)
    }

    pub fn patch(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Patch, url)
    }

    pub fn delete(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Delete, url)
    }

    pub fn head(&self, url: impl Into<String>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), RequestMethod::Head, url)
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }

    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    pub fn middlewares(&self) -> &MiddlewareChain {
        &self.middlewares
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for HTTP client
pub struct HttpClientBuilder {
    timeout: Option<Duration>,
    user_agent: Option<String>,
    default_headers: Vec<(String, String)>,
    middlewares: Vec<Box<dyn Middleware>>,
    retry_config: RetryConfig,
    verify_ssl: bool,
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self {
            timeout: None,
            user_agent: None,
            default_headers: Vec::new(),
            middlewares: Vec::new(),
            retry_config: RetryConfig::default(),
            verify_ssl: true,
        }
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.push((key.into(), value.into()));
        self
    }

    pub fn middleware(mut self, middleware: Box<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.verify_ssl = verify;
        self
    }

    pub fn build(self) -> anyhow::Result<HttpClient> {
        let mut client_builder = Client::builder();

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        if let Some(user_agent) = self.user_agent {
            client_builder = client_builder.user_agent(user_agent);
        }

        client_builder = client_builder.danger_accept_invalid_certs(!self.verify_ssl);

        let client = client_builder.build()?;

        let mut chain = MiddlewareChain::new();
        for middleware in self.middlewares {
            chain.add(middleware);
        }

        Ok(HttpClient {
            client,
            middlewares: Arc::new(chain),
            retry_config: self.retry_config,
        })
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
