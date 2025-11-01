//! Resource metadata and links

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Metadata for resources
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata {
    /// Additional metadata fields
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: String, value: serde_json::Value) {
        self.fields.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.fields.get(key)
    }
}

/// Builder for metadata
pub struct MetadataBuilder {
    metadata: Metadata,
}

impl MetadataBuilder {
    pub fn new() -> Self {
        Self {
            metadata: Metadata::new(),
        }
    }

    pub fn add<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        let json_value = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        self.metadata.insert(key.into(), json_value);
        self
    }

    pub fn build(self) -> Metadata {
        self.metadata
    }
}

impl Default for MetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// HATEOAS links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinks {
    #[serde(rename = "self")]
    pub self_link: String,
    #[serde(flatten)]
    pub additional: HashMap<String, String>,
}

impl ResourceLinks {
    pub fn new(self_link: impl Into<String>) -> Self {
        Self {
            self_link: self_link.into(),
            additional: HashMap::new(),
        }
    }

    pub fn add_link(mut self, rel: impl Into<String>, href: impl Into<String>) -> Self {
        self.additional.insert(rel.into(), href.into());
        self
    }
}
