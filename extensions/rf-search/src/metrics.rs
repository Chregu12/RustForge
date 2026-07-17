//! Metrics wrapper for search operations
//!
//! Provides automatic metrics collection for search driver operations.

use crate::driver::{Result, SearchDriver};
use crate::searchable::{SearchOptions, SearchResult, Searchable};
use async_trait::async_trait;
use std::sync::Arc;

/// Wrapper that adds metrics to any SearchDriver
pub struct MetricsWrapper<D: SearchDriver> {
    inner: Arc<D>,
}

impl<D: SearchDriver> MetricsWrapper<D> {
    /// Create a new metrics wrapper
    pub fn new(driver: D) -> Self {
        Self {
            inner: Arc::new(driver),
        }
    }

    /// Get a reference to the inner driver
    pub fn inner(&self) -> &D {
        &self.inner
    }
}

#[async_trait]
impl<D: SearchDriver + 'static> SearchDriver for MetricsWrapper<D> {
    async fn index<T: Searchable>(&self, document: &T) -> Result<()> {
        self.inner.index(document).await
    }

    async fn index_many<T: Searchable>(&self, documents: Vec<&T>) -> Result<()> {
        self.inner.index_many(documents).await
    }

    async fn search<T: Searchable>(
        &self,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<T::Model>> {
        #[cfg(feature = "metrics")]
        {
            rf_metrics::SEARCH_QUERIES.inc();
            let timer = rf_metrics::SEARCH_DURATION.start_timer();

            let result = self.inner.search::<T>(query, options).await;

            match &result {
                Ok(_) => {
                    // Success - timer will automatically record duration on drop
                }
                Err(_) => {
                    rf_metrics::SEARCH_ERRORS.inc();
                }
            }

            drop(timer);
            result
        }

        #[cfg(not(feature = "metrics"))]
        {
            self.inner.search::<T>(query, options).await
        }
    }

    async fn delete<T: Searchable>(&self, id: &str) -> Result<()> {
        self.inner.delete::<T>(id).await
    }

    async fn update<T: Searchable>(&self, document: &T) -> Result<()> {
        self.inner.update(document).await
    }

    async fn clear_index<T: Searchable>(&self) -> Result<()> {
        self.inner.clear_index::<T>().await
    }

    async fn count<T: Searchable>(&self) -> Result<usize> {
        self.inner.count::<T>().await
    }

    async fn health_check(&self) -> Result<()> {
        self.inner.health_check().await
    }
}

#[cfg(test)]
#[cfg(feature = "metrics")]
mod tests {
    use super::*;
    use crate::drivers::InMemoryDriver;
    use crate::searchable::Searchable;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestDoc {
        id: String,
        title: String,
    }

    impl Searchable for TestDoc {
        type Model = Self;

        fn search_id(&self) -> String {
            self.id.clone()
        }

        fn search_fields(&self) -> Vec<(String, String)> {
            vec![("title".to_string(), self.title.clone())]
        }

        fn from_search_result(_fields: Vec<(String, String)>) -> Self::Model {
            Self {
                id: "test".to_string(),
                title: "Test".to_string(),
            }
        }
    }

    #[tokio::test]
    async fn test_metrics_wrapper() {
        let driver = InMemoryDriver::new();
        let wrapped = MetricsWrapper::new(driver);

        let doc = TestDoc {
            id: "1".to_string(),
            title: "Test Document".to_string(),
        };

        // These operations should record metrics
        wrapped.index(&doc).await.unwrap();
        let _result = wrapped.search::<TestDoc>("Test", None).await.unwrap();
    }
}
