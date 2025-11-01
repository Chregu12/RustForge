//! Resource collections with pagination support

use crate::{Resource, ResourceContext, Pagination, PaginationMeta};
use serde::{Serialize, Deserialize};

/// Options for collection transformation
#[derive(Debug, Clone)]
pub struct CollectionOptions {
    /// Pagination settings
    pub pagination: Option<Pagination>,
    /// Resource context
    pub context: ResourceContext,
    /// Collection metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for CollectionOptions {
    fn default() -> Self {
        Self {
            pagination: None,
            context: ResourceContext::default(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// A collection of API resources with pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceCollection<T> {
    /// The resources
    pub data: Vec<T>,
    /// Pagination metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional: Option<serde_json::Value>,
}

impl<T> ResourceCollection<T>
where
    T: Resource + Serialize,
{
    /// Create a new collection
    pub fn new(data: Vec<T>) -> Self {
        Self {
            data,
            meta: None,
            additional: None,
        }
    }

    /// Create a collection from models
    pub fn from_models<M>(models: Vec<M>) -> Self
    where
        T: Resource<Model = M>,
    {
        let data = models.into_iter().map(T::from_model).collect();
        Self {
            data,
            meta: None,
            additional: None,
        }
    }

    /// Create a paginated collection
    pub fn paginated(data: Vec<T>, pagination: Pagination, total: u64) -> Self {
        let meta = PaginationMeta::from_pagination(&pagination, total);
        Self {
            data,
            meta: Some(meta),
            additional: None,
        }
    }

    /// Add metadata
    pub fn with_meta(mut self, meta: PaginationMeta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add additional data
    pub fn with_additional(mut self, additional: serde_json::Value) -> Self {
        self.additional = Some(additional);
        self
    }

    /// Get the number of items
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if collection is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Map the collection to another type
    pub fn map<F, U>(self, f: F) -> ResourceCollection<U>
    where
        F: FnMut(T) -> U,
        U: Serialize,
    {
        ResourceCollection {
            data: self.data.into_iter().map(f).collect(),
            meta: self.meta,
            additional: self.additional,
        }
    }

    /// Filter the collection
    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        self.data.retain(f);
        self
    }
}

impl<T> Default for ResourceCollection<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            meta: None,
            additional: None,
        }
    }
}

impl<T> From<Vec<T>> for ResourceCollection<T>
where
    T: Serialize,
{
    fn from(data: Vec<T>) -> Self {
        Self {
            data,
            meta: None,
            additional: None,
        }
    }
}

/// Builder for creating resource collections
pub struct CollectionBuilder<T> {
    data: Vec<T>,
    options: CollectionOptions,
}

impl<T> CollectionBuilder<T>
where
    T: Resource + Serialize,
{
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            options: CollectionOptions::default(),
        }
    }

    /// Add items to the collection
    pub fn items(mut self, items: Vec<T>) -> Self {
        self.data = items;
        self
    }

    /// Add pagination
    pub fn paginate(mut self, pagination: Pagination) -> Self {
        self.options.pagination = Some(pagination);
        self
    }

    /// Add context
    pub fn context(mut self, context: ResourceContext) -> Self {
        self.options.context = context;
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.options.metadata.insert(key, value);
        self
    }

    /// Build the collection
    pub fn build(self, total: Option<u64>) -> ResourceCollection<T> {
        let mut collection = ResourceCollection::new(self.data);

        if let Some(pagination) = self.options.pagination {
            if let Some(total) = total {
                collection.meta = Some(PaginationMeta::from_pagination(&pagination, total));
            }
        }

        if !self.options.metadata.is_empty() {
            collection.additional = Some(serde_json::json!(self.options.metadata));
        }

        collection
    }
}

impl<T> Default for CollectionBuilder<T>
where
    T: Resource + Serialize,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestResource {
        id: i32,
    }

    impl Resource for TestResource {
        type Model = i32;

        fn from_model(model: Self::Model) -> Self {
            Self { id: model }
        }
    }

    #[test]
    fn test_collection_creation() {
        let items = vec![TestResource { id: 1 }, TestResource { id: 2 }];
        let collection = ResourceCollection::new(items);
        assert_eq!(collection.len(), 2);
    }

    #[test]
    fn test_from_models() {
        let collection = ResourceCollection::<TestResource>::from_models(vec![1, 2, 3]);
        assert_eq!(collection.len(), 3);
    }

    #[test]
    fn test_builder() {
        let items = vec![TestResource { id: 1 }];
        let collection = CollectionBuilder::new()
            .items(items)
            .build(Some(1));

        assert_eq!(collection.len(), 1);
    }
}
