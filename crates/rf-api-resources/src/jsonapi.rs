use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: serde_json::Value,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub relationships: HashMap<String, JsonApiRelationship>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<JsonApiLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl JsonApiResource {
    pub fn new(
        id: impl Into<String>,
        resource_type: impl Into<String>,
        attributes: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            resource_type: resource_type.into(),
            attributes,
            relationships: HashMap::new(),
            links: None,
            meta: None,
        }
    }

    pub fn with_relationship(
        mut self,
        name: impl Into<String>,
        relationship: JsonApiRelationship,
    ) -> Self {
        self.relationships.insert(name.into(), relationship);
        self
    }

    pub fn with_links(mut self, links: JsonApiLinks) -> Self {
        self.links = Some(links);
        self
    }

    pub fn with_meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiDocument {
    pub data: JsonApiData,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub included: Vec<JsonApiResource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<JsonApiLinks>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    pub jsonapi: JsonApiVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonApiData {
    Single(Box<JsonApiResource>),
    Multiple(Vec<JsonApiResource>),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiLinks {
    #[serde(rename = "self", skip_serializing_if = "Option::is_none")]
    pub self_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl JsonApiLinks {
    pub fn new() -> Self {
        Self {
            self_link: None,
            first: None,
            last: None,
            prev: None,
            next: None,
        }
    }

    pub fn self_link(mut self, url: impl Into<String>) -> Self {
        self.self_link = Some(url.into());
        self
    }

    pub fn first(mut self, url: impl Into<String>) -> Self {
        self.first = Some(url.into());
        self
    }

    pub fn last(mut self, url: impl Into<String>) -> Self {
        self.last = Some(url.into());
        self
    }

    pub fn prev(mut self, url: impl Into<String>) -> Self {
        self.prev = Some(url.into());
        self
    }

    pub fn next(mut self, url: impl Into<String>) -> Self {
        self.next = Some(url.into());
        self
    }
}

impl Default for JsonApiLinks {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiVersion {
    pub version: String,
}

impl JsonApiVersion {
    pub fn v1() -> Self {
        Self {
            version: "1.0".to_string(),
        }
    }
}

impl Default for JsonApiVersion {
    fn default() -> Self {
        Self::v1()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiRelationship {
    pub data: Option<JsonApiRelationshipData>,
    pub links: Option<JsonApiLinks>,
}

impl JsonApiRelationship {
    pub fn to_one(id: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self {
            data: Some(JsonApiRelationshipData::Single(JsonApiResourceIdentifier {
                id: id.into(),
                resource_type: resource_type.into(),
            })),
            links: None,
        }
    }

    pub fn to_many(items: Vec<JsonApiResourceIdentifier>) -> Self {
        Self {
            data: Some(JsonApiRelationshipData::Multiple(items)),
            links: None,
        }
    }

    pub fn empty() -> Self {
        Self {
            data: None,
            links: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonApiRelationshipData {
    Single(JsonApiResourceIdentifier),
    Multiple(Vec<JsonApiResourceIdentifier>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonApiResourceIdentifier {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
}

impl JsonApiResourceIdentifier {
    pub fn new(id: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resource_type: resource_type.into(),
        }
    }
}

/// Trait for models that can be serialized to JSON:API format.
pub trait JsonApiSerializable {
    fn resource_type() -> &'static str
    where
        Self: Sized;
    fn to_jsonapi(&self) -> JsonApiResource;
}

/// Sparse fieldsets: return only specific fields per resource type.
pub struct SparseFieldset {
    fields: HashMap<String, Vec<String>>,
}

impl SparseFieldset {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn add(mut self, resource_type: &str, fields: Vec<&str>) -> Self {
        self.fields.insert(
            resource_type.to_string(),
            fields.into_iter().map(|f| f.to_string()).collect(),
        );
        self
    }

    /// Filter a resource's attributes to only include the specified fields.
    /// If no fieldset is configured for the resource type, all attributes are kept.
    pub fn filter_resource(&self, resource: &mut JsonApiResource) {
        if let Some(allowed_fields) = self.fields.get(&resource.resource_type) {
            if let serde_json::Value::Object(ref mut map) = resource.attributes {
                let keys_to_remove: Vec<String> = map
                    .keys()
                    .filter(|k| !allowed_fields.contains(k))
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    map.remove(&key);
                }
            }
        }
    }
}

impl Default for SparseFieldset {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing JSON:API response documents.
pub struct JsonApiResponseBuilder {
    data: Option<JsonApiData>,
    included: Vec<JsonApiResource>,
    meta: Option<serde_json::Value>,
    links: Option<JsonApiLinks>,
}

impl JsonApiResponseBuilder {
    pub fn new() -> Self {
        Self {
            data: None,
            included: Vec::new(),
            meta: None,
            links: None,
        }
    }

    pub fn data(mut self, resource: JsonApiResource) -> Self {
        self.data = Some(JsonApiData::Single(Box::new(resource)));
        self
    }

    pub fn collection(mut self, resources: Vec<JsonApiResource>) -> Self {
        self.data = Some(JsonApiData::Multiple(resources));
        self
    }

    pub fn null(mut self) -> Self {
        self.data = Some(JsonApiData::Null);
        self
    }

    pub fn include(mut self, resource: JsonApiResource) -> Self {
        self.included.push(resource);
        self
    }

    pub fn meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn links(mut self, links: JsonApiLinks) -> Self {
        self.links = Some(links);
        self
    }

    pub fn build(self) -> JsonApiDocument {
        JsonApiDocument {
            data: self.data.unwrap_or(JsonApiData::Null),
            included: self.included,
            links: self.links,
            meta: self.meta,
            jsonapi: JsonApiVersion::v1(),
        }
    }
}

impl Default for JsonApiResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
