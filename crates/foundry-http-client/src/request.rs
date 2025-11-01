//! Request builder implementation

use crate::auth::{Auth, AuthType};
use crate::response::Response;
use crate::HttpClient;
use reqwest::{header, Method};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum RequestMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
}

impl From<RequestMethod> for Method {
    fn from(method: RequestMethod) -> Self {
        match method {
            RequestMethod::Get => Method::GET,
            RequestMethod::Post => Method::POST,
            RequestMethod::Put => Method::PUT,
            RequestMethod::Patch => Method::PATCH,
            RequestMethod::Delete => Method::DELETE,
            RequestMethod::Head => Method::HEAD,
        }
    }
}

/// Fluent request builder
pub struct RequestBuilder {
    client: HttpClient,
    method: RequestMethod,
    url: String,
    headers: HashMap<String, String>,
    query: Vec<(String, String)>,
    body: Option<RequestBody>,
    auth: Option<Auth>,
    timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
enum RequestBody {
    Json(serde_json::Value),
    Form(HashMap<String, String>),
    Text(String),
    Bytes(Vec<u8>),
}

impl RequestBuilder {
    pub(crate) fn new(client: HttpClient, method: RequestMethod, url: impl Into<String>) -> Self {
        Self {
            client,
            method,
            url: url.into(),
            headers: HashMap::new(),
            query: Vec::new(),
            body: None,
            auth: None,
            timeout: None,
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers.extend(headers);
        self
    }

    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    pub fn queries(mut self, queries: Vec<(String, String)>) -> Self {
        self.query.extend(queries);
        self
    }

    pub fn json<T: Serialize>(mut self, body: &T) -> anyhow::Result<Self> {
        let value = serde_json::to_value(body)?;
        self.body = Some(RequestBody::Json(value));
        Ok(self)
    }

    pub fn form(mut self, form: HashMap<String, String>) -> Self {
        self.body = Some(RequestBody::Form(form));
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.body = Some(RequestBody::Text(text.into()));
        self
    }

    pub fn bytes(mut self, bytes: Vec<u8>) -> Self {
        self.body = Some(RequestBody::Bytes(bytes));
        self
    }

    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(Auth::new(AuthType::Bearer(token.into())));
        self
    }

    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.auth = Some(Auth::new(AuthType::Basic {
            username: username.into(),
            password: password.into(),
        }));
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    pub async fn send(self) -> anyhow::Result<Response> {
        let mut request = self.client.inner().request(
            Method::from(self.method),
            &self.url,
        );

        // Add headers
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // Add query parameters
        if !self.query.is_empty() {
            request = request.query(&self.query);
        }

        // Add body
        if let Some(body) = self.body {
            request = match body {
                RequestBody::Json(json) => {
                    request.header(header::CONTENT_TYPE, "application/json").json(&json)
                }
                RequestBody::Form(form) => request.form(&form),
                RequestBody::Text(text) => {
                    request.header(header::CONTENT_TYPE, "text/plain").body(text)
                }
                RequestBody::Bytes(bytes) => {
                    request.header(header::CONTENT_TYPE, "application/octet-stream").body(bytes)
                }
            };
        }

        // Add authentication
        if let Some(auth) = self.auth {
            request = auth.apply(request)?;
        }

        // Add timeout
        if let Some(timeout) = self.timeout {
            request = request.timeout(timeout);
        }

        // Execute with retry logic
        let retry_config = self.client.retry_config();
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= retry_config.max_retries {
            match request.try_clone() {
                Some(req) => {
                    match req.send().await {
                        Ok(response) => {
                            return Ok(Response::new(response));
                        }
                        Err(e) => {
                            last_error = Some(e);
                            attempts += 1;
                            if attempts <= retry_config.max_retries {
                                tokio::time::sleep(retry_config.delay).await;
                            }
                        }
                    }
                }
                None => {
                    return Err(anyhow::anyhow!("Cannot clone request for retry"));
                }
            }
        }

        Err(anyhow::anyhow!(
            "Request failed after {} attempts: {:?}",
            attempts,
            last_error
        ))
    }
}
