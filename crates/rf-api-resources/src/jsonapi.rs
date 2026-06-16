//! JSON:API resource support (Laravel 13 style).
//!
//! This module produces standard [JSON:API](https://jsonapi.org/) documents of the shape:
//!
//! ```json
//! {
//!   "data": {
//!     "type": "...",
//!     "id": "...",
//!     "attributes": { },
//!     "relationships": { },
//!     "links": { }
//!   },
//!   "included": [ ],
//!   "meta": { },
//!   "links": { }
//! }
//! ```
//!
//! It is additive and does not replace the existing [`crate::Resource`] /
//! [`crate::ResourceCollection`] traits.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// An ordered map of relationship name to [`Relationship`].
///
/// A [`BTreeMap`] is used (rather than [`serde_json::Map`], which is fixed to a
/// `Value` value type) so the container can derive `Deserialize`/`PartialEq`
/// while keeping deterministic key ordering.
pub type RelationshipMap = BTreeMap<String, Relationship>;

/// A JSON:API resource identifier object: `{ "type": ..., "id": ... }`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceIdentifier {
    /// The resource type.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// The resource id.
    pub id: String,
}

impl ResourceIdentifier {
    /// Create a new resource identifier.
    pub fn new(resource_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }
}

/// The `data` member of a relationship: either a single linkage (or null) or many.
///
/// Serialized untagged so a to-one linkage is a single object (or `null`) and a
/// to-many linkage is an array.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RelationshipData {
    /// To-one linkage (or `null` when absent).
    One(Option<ResourceIdentifier>),
    /// To-many linkage.
    Many(Vec<ResourceIdentifier>),
}

/// A JSON:API relationship object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    /// Resource linkage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RelationshipData>,
    /// Relationship links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Map<String, Value>>,
}

impl Relationship {
    /// Build a to-one relationship from a single (optional) identifier.
    pub fn to_one(identifier: Option<ResourceIdentifier>) -> Self {
        Self {
            data: Some(RelationshipData::One(identifier)),
            links: None,
        }
    }

    /// Build a to-many relationship from a list of identifiers.
    pub fn to_many(identifiers: Vec<ResourceIdentifier>) -> Self {
        Self {
            data: Some(RelationshipData::Many(identifiers)),
            links: None,
        }
    }

    /// Attach a single link to the relationship.
    pub fn with_link(mut self, name: impl Into<String>, href: impl Into<String>) -> Self {
        let links = self.links.get_or_insert_with(Map::new);
        links.insert(name.into(), Value::String(href.into()));
        self
    }
}

/// A JSON:API resource object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceObject {
    /// The resource type.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// The resource id.
    pub id: String,
    /// Resource attributes (omitted when empty).
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub attributes: Map<String, Value>,
    /// Resource relationships.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<RelationshipMap>,
    /// Resource-level links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Map<String, Value>>,
}

/// The primary `data` of a document: a single resource object or many.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PrimaryData {
    /// A single resource.
    Single(Box<ResourceObject>),
    /// A collection of resources.
    Many(Vec<ResourceObject>),
}

/// A top-level JSON:API document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonApiDocument {
    /// The document's primary data.
    pub data: PrimaryData,
    /// Compound document included resources.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub included: Vec<ResourceObject>,
    /// Top-level meta information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    /// Top-level links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Map<String, Value>>,
}

impl JsonApiDocument {
    /// Build a document with a single primary resource.
    pub fn single(object: ResourceObject) -> Self {
        Self {
            data: PrimaryData::Single(Box::new(object)),
            included: Vec::new(),
            meta: None,
            links: None,
        }
    }

    /// Build a document with a collection of primary resources.
    pub fn collection(objects: Vec<ResourceObject>) -> Self {
        Self {
            data: PrimaryData::Many(objects),
            included: Vec::new(),
            meta: None,
            links: None,
        }
    }

    /// Set the top-level `meta` member.
    pub fn with_meta(mut self, meta: Value) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Add a top-level link.
    pub fn with_link(mut self, name: impl Into<String>, href: impl Into<String>) -> Self {
        let links = self.links.get_or_insert_with(Map::new);
        links.insert(name.into(), Value::String(href.into()));
        self
    }

    /// Append included (compound document) resources.
    pub fn with_included(mut self, mut included: Vec<ResourceObject>) -> Self {
        self.included.append(&mut included);
        self
    }
}

/// A type that can be represented as a JSON:API resource.
///
/// # Example
///
/// ```rust
/// use rf_api_resources::{JsonApiResource, Relationship, ResourceIdentifier};
/// use rf_api_resources::jsonapi::RelationshipMap;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Article {
///     id: u64,
///     title: String,
/// }
///
/// impl JsonApiResource for Article {
///     fn json_api_type() -> &'static str {
///         "articles"
///     }
///     fn json_api_id(&self) -> String {
///         self.id.to_string()
///     }
///     fn relationships(&self) -> Option<RelationshipMap> {
///         let mut map = RelationshipMap::new();
///         map.insert(
///             "author".to_string(),
///             Relationship::to_one(Some(ResourceIdentifier::new("people", "9"))),
///         );
///         Some(map)
///     }
/// }
///
/// let article = Article { id: 1, title: "Hello".into() };
/// let doc = article.to_document();
/// let json = serde_json::to_value(&doc).unwrap();
///
/// assert_eq!(json["data"]["type"], "articles");
/// assert_eq!(json["data"]["id"], "1");
/// assert_eq!(json["data"]["attributes"]["title"], "Hello");
/// // The `id` is not duplicated into attributes.
/// assert!(json["data"]["attributes"].get("id").is_none());
/// assert_eq!(json["data"]["relationships"]["author"]["data"]["type"], "people");
/// ```
pub trait JsonApiResource: Serialize {
    /// The JSON:API resource type (e.g. `"articles"`).
    fn json_api_type() -> &'static str;

    /// The JSON:API resource id as a string.
    fn json_api_id(&self) -> String;

    /// The resource attributes.
    ///
    /// The default implementation serializes `self` to a JSON object and drops
    /// the `"id"` key (if present) so it is not duplicated alongside the
    /// top-level `id`. Non-object serializations yield an empty map.
    fn attributes(&self) -> Map<String, Value> {
        match serde_json::to_value(self) {
            Ok(Value::Object(mut map)) => {
                map.remove("id");
                map
            }
            _ => Map::new(),
        }
    }

    /// The resource relationships, if any.
    fn relationships(&self) -> Option<RelationshipMap> {
        None
    }

    /// The resource's `self` link, if any.
    fn self_link(&self) -> Option<String> {
        None
    }

    /// Build the [`ResourceObject`] for this resource.
    fn to_resource_object(&self) -> ResourceObject {
        let links = self.self_link().map(|href| {
            let mut map = Map::new();
            map.insert("self".to_string(), Value::String(href));
            map
        });

        ResourceObject {
            resource_type: Self::json_api_type().to_string(),
            id: self.json_api_id(),
            attributes: self.attributes(),
            relationships: self.relationships(),
            links,
        }
    }

    /// Build a single-resource JSON:API document for this resource.
    fn to_document(&self) -> JsonApiDocument {
        JsonApiDocument::single(self.to_resource_object())
    }
}

/// Build a collection JSON:API document from a slice of resources.
///
/// # Example
///
/// ```rust
/// use rf_api_resources::{document_from_collection, JsonApiResource};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Tag {
///     id: u64,
///     name: String,
/// }
///
/// impl JsonApiResource for Tag {
///     fn json_api_type() -> &'static str { "tags" }
///     fn json_api_id(&self) -> String { self.id.to_string() }
/// }
///
/// let tags = vec![
///     Tag { id: 1, name: "rust".into() },
///     Tag { id: 2, name: "api".into() },
/// ];
/// let doc = document_from_collection(&tags);
/// let json = serde_json::to_value(&doc).unwrap();
///
/// assert!(json["data"].is_array());
/// assert_eq!(json["data"][0]["attributes"]["name"], "rust");
/// ```
pub fn document_from_collection<R: JsonApiResource>(items: &[R]) -> JsonApiDocument {
    JsonApiDocument::collection(items.iter().map(|i| i.to_resource_object()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    struct Article {
        id: u64,
        title: String,
        body: String,
    }

    impl JsonApiResource for Article {
        fn json_api_type() -> &'static str {
            "articles"
        }

        fn json_api_id(&self) -> String {
            self.id.to_string()
        }

        fn relationships(&self) -> Option<RelationshipMap> {
            let mut map = RelationshipMap::new();
            map.insert(
                "author".to_string(),
                Relationship::to_one(Some(ResourceIdentifier::new("people", "9"))),
            );
            map.insert(
                "comments".to_string(),
                Relationship::to_many(vec![
                    ResourceIdentifier::new("comments", "5"),
                    ResourceIdentifier::new("comments", "12"),
                ]),
            );
            Some(map)
        }

        fn self_link(&self) -> Option<String> {
            Some(format!("/articles/{}", self.id))
        }
    }

    fn sample() -> Article {
        Article {
            id: 1,
            title: "JSON:API in Rust".to_string(),
            body: "Body text".to_string(),
        }
    }

    #[test]
    fn test_single_document_shape() {
        let doc = sample().to_document();
        let json = serde_json::to_value(&doc).unwrap();

        assert_eq!(json["data"]["type"], "articles");
        assert_eq!(json["data"]["id"], "1");
        assert_eq!(json["data"]["attributes"]["title"], "JSON:API in Rust");
        assert_eq!(json["data"]["attributes"]["body"], "Body text");
        // id must not be duplicated into attributes.
        assert!(json["data"]["attributes"].get("id").is_none());
    }

    #[test]
    fn test_relationships() {
        let doc = sample().to_document();
        let json = serde_json::to_value(&doc).unwrap();

        // to-one: single object.
        assert_eq!(json["data"]["relationships"]["author"]["data"]["type"], "people");
        assert_eq!(json["data"]["relationships"]["author"]["data"]["id"], "9");

        // to-many: array.
        let comments = &json["data"]["relationships"]["comments"]["data"];
        assert!(comments.is_array());
        assert_eq!(comments.as_array().unwrap().len(), 2);
        assert_eq!(comments[0]["type"], "comments");
        assert_eq!(comments[1]["id"], "12");
    }

    #[test]
    fn test_self_link() {
        let doc = sample().to_document();
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["data"]["links"]["self"], "/articles/1");
    }

    #[test]
    fn test_to_one_null() {
        let rel = Relationship::to_one(None);
        let json = serde_json::to_value(&rel).unwrap();
        assert!(json["data"].is_null());
    }

    #[test]
    fn test_document_with_meta_links_included() {
        let included = vec![ResourceObject {
            resource_type: "people".to_string(),
            id: "9".to_string(),
            attributes: {
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String("Dan".to_string()));
                m
            },
            relationships: None,
            links: None,
        }];

        let doc = sample()
            .to_document()
            .with_included(included)
            .with_meta(serde_json::json!({ "count": 1 }))
            .with_link("self", "/articles/1");

        let json = serde_json::to_value(&doc).unwrap();

        assert_eq!(json["meta"]["count"], 1);
        assert_eq!(json["links"]["self"], "/articles/1");
        assert!(json["included"].is_array());
        assert_eq!(json["included"][0]["type"], "people");
        assert_eq!(json["included"][0]["attributes"]["name"], "Dan");
    }

    #[test]
    fn test_collection_document() {
        let items = vec![
            sample(),
            Article {
                id: 2,
                title: "Second".to_string(),
                body: "More".to_string(),
            },
        ];

        let doc = document_from_collection(&items);
        let json = serde_json::to_value(&doc).unwrap();

        assert!(json["data"].is_array());
        let arr = json["data"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "1");
        assert_eq!(arr[1]["id"], "2");
        assert_eq!(arr[1]["attributes"]["title"], "Second");
        // No empty top-level members serialized.
        assert!(json.get("included").is_none());
        assert!(json.get("meta").is_none());
        assert!(json.get("links").is_none());
    }

    #[test]
    fn test_empty_attributes_omitted() {
        #[derive(Serialize)]
        struct Bare {
            id: u64,
        }
        impl JsonApiResource for Bare {
            fn json_api_type() -> &'static str {
                "bare"
            }
            fn json_api_id(&self) -> String {
                self.id.to_string()
            }
        }

        let json = serde_json::to_value(&Bare { id: 7 }.to_document()).unwrap();
        assert_eq!(json["data"]["type"], "bare");
        assert!(json["data"].get("attributes").is_none());
        assert!(json["data"].get("relationships").is_none());
    }

    #[test]
    fn test_roundtrip_deserialize() {
        let doc = sample().to_document();
        let json = serde_json::to_string(&doc).unwrap();
        let back: JsonApiDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }
}
