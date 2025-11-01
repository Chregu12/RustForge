//! HTTP response wrapper

use reqwest::StatusCode;
use serde::de::DeserializeOwned;

/// Response wrapper
pub struct Response {
    inner: reqwest::Response,
}

impl Response {
    pub(crate) fn new(inner: reqwest::Response) -> Self {
        Self { inner }
    }

    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    pub fn is_success(&self) -> bool {
        self.inner.status().is_success()
    }

    pub fn is_client_error(&self) -> bool {
        self.inner.status().is_client_error()
    }

    pub fn is_server_error(&self) -> bool {
        self.inner.status().is_server_error()
    }

    pub fn headers(&self) -> &reqwest::header::HeaderMap {
        self.inner.headers()
    }

    pub async fn text(self) -> anyhow::Result<String> {
        Ok(self.inner.text().await?)
    }

    pub async fn bytes(self) -> anyhow::Result<Vec<u8>> {
        Ok(self.inner.bytes().await?.to_vec())
    }

    pub async fn json<T: DeserializeOwned>(self) -> anyhow::Result<T> {
        Ok(self.inner.json().await?)
    }

    pub fn error_for_status(self) -> anyhow::Result<Self> {
        let inner = self.inner.error_for_status()?;
        Ok(Self { inner })
    }
}
