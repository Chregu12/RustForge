//! Algolia search driver for rf-search
//!
//! Provides integration with Algolia's hosted search service.

use crate::driver::{ConfigurableDriver, Result, SearchDriver, SearchError};
use crate::searchable::{SearchHit, SearchOptions, SearchResult, Searchable};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

// Only compile when algolia feature is enabled
#[cfg(feature = "algolia")]
use reqwest::Client;

/// Algolia search driver configuration
#[derive(Clone)]
pub struct AlgoliaConfig {
    /// Algolia Application ID
    pub app_id: String,

    /// Algolia API Key
    pub api_key: String,

    /// Base API URL (defaults to https://APP_ID-dsn.algolia.net)
    pub api_url: Option<String>,

    /// Timeout for requests in seconds
    pub timeout_secs: u64,
}

impl AlgoliaConfig {
    /// Create a new Algolia configuration
    pub fn new(app_id: impl Into<String>, api_key: impl Into<String>) -> Self {
        let app_id = app_id.into();
        Self {
            api_url: Some(format!("https://{}-dsn.algolia.net", app_id)),
            app_id,
            api_key: api_key.into(),
            timeout_secs: 30,
        }
    }

    /// Set custom API URL
    pub fn api_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = Some(url.into());
        self
    }

    /// Set request timeout
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// Algolia search driver
pub struct AlgoliaDriver {
    config: AlgoliaConfig,
    client: Client,
}

impl AlgoliaDriver {
    /// Create a new Algolia driver
    pub fn new(config: AlgoliaConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// Get base URL for API requests
    fn base_url(&self) -> String {
        self.config
            .api_url
            .clone()
            .unwrap_or_else(|| format!("https://{}-dsn.algolia.net", self.config.app_id))
    }

    /// Build index URL
    fn index_url(&self, index: &str) -> String {
        format!("{}/1/indexes/{}", self.base_url(), index)
    }

    /// Execute a request with Algolia headers
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<Value>,
    ) -> Result<T> {
        let mut request = self
            .client
            .request(method, url)
            .header("X-Algolia-Application-Id", &self.config.app_id)
            .header("X-Algolia-API-Key", &self.config.api_key)
            .header("Content-Type", "application/json");

        if let Some(body_data) = body {
            request = request.json(&body_data);
        }

        let response = request
            .send()
            .await
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SearchError::QueryError(format!(
                "Algolia error {}: {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| SearchError::QueryError(e.to_string()))
    }
}

#[async_trait]
impl SearchDriver for AlgoliaDriver {
    async fn index<T: Searchable>(&self, item: &T) -> Result<()> {
        let index_name = T::index_name();
        let url = format!("{}/{}", self.index_url(index_name), item.search_id());

        let data = serde_json::to_value(&item.to_searchable())
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        let _: Value = self.request(reqwest::Method::PUT, &url, Some(data)).await?;

        Ok(())
    }

    async fn index_many<T: Searchable>(
        &self,
        items: Vec<&T>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let index_name = T::index_name();
        let url = format!("{}/batch", self.index_url(index_name));

        let requests: Vec<Value> = items
            .iter()
            .map(|item| {
                let mut obj = serde_json::to_value(&item.to_searchable()).unwrap_or_default();
                if let Some(obj_map) = obj.as_object_mut() {
                    obj_map.insert("objectID".to_string(), Value::String(item.search_id()));
                }
                json!({
                    "action": "addObject",
                    "body": obj
                })
            })
            .collect();

        let body = json!({ "requests": requests });

        let _: Value = self
            .request(reqwest::Method::POST, &url, Some(body))
            .await?;

        Ok(())
    }

    async fn delete<T: Searchable>(&self, id: &str) -> Result<()> {
        let index_name = T::index_name();
        let url = format!("{}/{}", self.index_url(index_name), id);

        let _: Value = self.request(reqwest::Method::DELETE, &url, None).await?;

        Ok(())
    }

    async fn search<T: Searchable>(
        &self,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<T::Model>> {
        let index_name = T::index_name();
        let url = format!("{}/query", self.index_url(index_name));

        let opts = options.unwrap_or_default();
        let limit = if opts.limit == 0 { 20 } else { opts.limit };
        let page = if limit > 0 { opts.offset / limit } else { 0 };

        let mut search_params = json!({
            "query": query,
            "hitsPerPage": limit,
            "page": page,
        });

        if !opts.filters.is_empty() {
            let filter_str = opts
                .filters
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v))
                .collect::<Vec<_>>()
                .join(" AND ");
            search_params["filters"] = Value::String(filter_str);
        }

        if let Some((sort_field, _asc)) = opts.sort {
            search_params["sortBy"] = Value::String(sort_field);
        }

        if !opts.highlight_fields.is_empty() {
            search_params["attributesToHighlight"] = json!(opts.highlight_fields);
        }

        #[derive(Deserialize)]
        struct AlgoliaResponse<M> {
            hits: Vec<AlgoliaHit<M>>,
            #[serde(rename = "nbHits")]
            nb_hits: usize,
            page: usize,
            #[serde(rename = "processingTimeMS", default)]
            processing_time_ms: u64,
        }

        #[derive(Deserialize)]
        struct AlgoliaHit<M> {
            #[serde(rename = "objectID")]
            object_id: String,
            #[serde(flatten)]
            document: M,
            #[serde(rename = "_highlightResult", default)]
            highlight_result: Option<HashMap<String, HighlightField>>,
        }

        #[derive(Deserialize)]
        struct HighlightField {
            value: String,
        }

        let response: AlgoliaResponse<T::Model> = self
            .request(reqwest::Method::POST, &url, Some(search_params))
            .await?;

        let total = response.nb_hits;
        let hits: Vec<SearchHit<T::Model>> = response
            .hits
            .into_iter()
            .enumerate()
            .map(|(idx, hit)| {
                let score = if total > 0 {
                    ((total - idx) as f64) / (total as f64)
                } else {
                    0.0
                };
                let highlights: Option<HashMap<String, Vec<String>>> = hit
                    .highlight_result
                    .map(|hr| hr.into_iter().map(|(k, v)| (k, vec![v.value])).collect());
                SearchHit {
                    document: hit.document,
                    score,
                    highlights,
                    metadata: None,
                }
            })
            .collect();

        Ok(SearchResult {
            hits,
            total,
            query: query.to_string(),
            processing_time_ms: response.processing_time_ms,
            page: Some(response.page),
            per_page: Some(limit),
        })
    }

    async fn clear_index<T: Searchable>(&self) -> Result<()> {
        let index_name = T::index_name();
        let url = format!("{}/clear", self.index_url(index_name));

        let _: Value = self.request(reqwest::Method::POST, &url, None).await?;

        Ok(())
    }

    async fn count<T: Searchable>(&self) -> Result<usize> {
        let index_name = T::index_name();
        let url = format!("{}/query", self.index_url(index_name));

        // Query with limit 0 just to get total count
        let params = json!({"query": "", "hitsPerPage": 0});

        #[derive(Deserialize)]
        struct CountResponse {
            #[serde(rename = "nbHits")]
            nb_hits: usize,
        }

        let response: CountResponse = self
            .request(reqwest::Method::POST, &url, Some(params))
            .await?;

        Ok(response.nb_hits)
    }

    async fn health_check(&self) -> Result<()> {
        let url = format!("{}/1/indexes", self.base_url());

        // Try to list indexes as a health check
        let _: Value = self.request(reqwest::Method::GET, &url, None).await?;

        Ok(())
    }
}

#[async_trait]
impl ConfigurableDriver for AlgoliaDriver {
    async fn configure_index<T: Searchable>(&self) -> Result<()> {
        // Algolia indexes are created automatically on first insert.
        // We apply searchable attribute settings here.
        let index_name = T::index_name();
        let url = format!("{}/settings", self.index_url(index_name));

        let settings = json!({
            "searchableAttributes": T::searchable_fields(),
            "attributesToRetrieve": ["*"],
        });

        let _: Value = self
            .request(reqwest::Method::PUT, &url, Some(settings))
            .await?;

        Ok(())
    }

    async fn set_searchable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()> {
        let index_name = T::index_name();
        let url = format!("{}/settings", self.index_url(index_name));
        let settings = json!({ "searchableAttributes": fields });
        let _: Value = self.request(reqwest::Method::PUT, &url, Some(settings)).await?;
        Ok(())
    }

    async fn set_filterable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()> {
        let index_name = T::index_name();
        let url = format!("{}/settings", self.index_url(index_name));
        let settings = json!({ "attributesForFaceting": fields });
        let _: Value = self.request(reqwest::Method::PUT, &url, Some(settings)).await?;
        Ok(())
    }

    async fn set_sortable_fields<T: Searchable>(&self, _fields: Vec<String>) -> Result<()> {
        // Algolia sorting is configured via replica indexes; not a simple settings call.
        // Implementing as a no-op for now.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestDoc {
        id: String,
        title: String,
        content: String,
    }

    impl Searchable for TestDoc {
        type Model = TestDoc;

        fn searchable_fields() -> Vec<&'static str> {
            vec!["title", "content"]
        }

        fn to_searchable(&self) -> Self::Model {
            self.clone()
        }

        fn search_id(&self) -> String {
            self.id.clone()
        }

        fn index_name() -> &'static str {
            "test_docs"
        }
    }

    #[test]
    fn test_algolia_config() {
        let config = AlgoliaConfig::new("TEST_APP_ID", "test_key").timeout(60);

        assert_eq!(config.app_id, "TEST_APP_ID");
        assert_eq!(config.timeout_secs, 60);
        assert!(config.api_url.unwrap().contains("TEST_APP_ID"));
    }

    #[test]
    fn test_algolia_driver_creation() {
        let config = AlgoliaConfig::new("TEST_APP_ID", "test_key");
        let driver = AlgoliaDriver::new(config);

        assert!(driver.base_url().contains("TEST_APP_ID"));
    }

    // Integration tests would require actual Algolia credentials
    // and should be run separately with #[ignore] or feature flags
}
