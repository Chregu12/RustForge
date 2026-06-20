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

    // --- Adversarial JSON:API shape tests (Feature 3 validation) ---

    #[test]
    fn test_type_and_id_are_json_strings() {
        let json = serde_json::to_value(&sample().to_document()).unwrap();
        // Per JSON:API, both `type` and `id` MUST be strings (not numbers).
        assert!(json["data"]["type"].is_string());
        assert!(json["data"]["id"].is_string());
        // Even though Article.id is a u64, the document id is "1" (string).
        assert_eq!(json["data"]["id"].as_str(), Some("1"));
    }

    #[test]
    fn test_attributes_is_object_without_id_but_keeps_other_fields() {
        let json = serde_json::to_value(&sample().to_document()).unwrap();
        let attrs = &json["data"]["attributes"];
        assert!(attrs.is_object());
        // Default attributes() drops only "id", not other fields.
        assert!(attrs.get("id").is_none());
        assert!(attrs.get("title").is_some());
        assert!(attrs.get("body").is_some());
    }

    #[test]
    fn test_attributes_only_drops_id_key_not_id_substrings() {
        // A field literally named "id" is dropped; "identifier" must survive.
        #[derive(Serialize)]
        struct Weird {
            id: u64,
            identifier: String,
        }
        impl JsonApiResource for Weird {
            fn json_api_type() -> &'static str {
                "weird"
            }
            fn json_api_id(&self) -> String {
                self.id.to_string()
            }
        }
        let json = serde_json::to_value(
            &Weird {
                id: 3,
                identifier: "keep-me".into(),
            }
            .to_document(),
        )
        .unwrap();
        assert!(json["data"]["attributes"].get("id").is_none());
        assert_eq!(json["data"]["attributes"]["identifier"], "keep-me");
        assert_eq!(json["data"]["id"], "3");
    }

    #[test]
    fn test_to_one_present_shape() {
        let rel = Relationship::to_one(Some(ResourceIdentifier::new("people", "9")));
        let json = serde_json::to_value(&rel).unwrap();
        // to-one present: data is a single object {type,id}.
        assert!(json["data"].is_object());
        assert_eq!(json["data"]["type"], "people");
        assert_eq!(json["data"]["id"], "9");
        // No links member when none set.
        assert!(json.get("links").is_none());
    }

    #[test]
    fn test_to_many_empty_is_empty_array_not_null() {
        let rel = Relationship::to_many(vec![]);
        let json = serde_json::to_value(&rel).unwrap();
        // to-many with no members must serialize as [] (array), not null.
        // NOTE: untagged enum -> an empty Vec matches `One(None)` shape? Verify.
        assert!(
            json["data"].is_array(),
            "empty to-many must be an array, got: {}",
            json["data"]
        );
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_empty_to_many_roundtrips_as_many() {
        // {"data":[]} must deserialize back to Many(vec![]), not be mis-parsed by
        // the untagged enum as a to-one.
        let rel = Relationship::to_many(vec![]);
        let s = serde_json::to_string(&rel).unwrap();
        let back: Relationship = serde_json::from_str(&s).unwrap();
        match back.data {
            Some(RelationshipData::Many(v)) => assert!(v.is_empty()),
            other => panic!("expected Many([]), got {:?}", other),
        }
    }

    #[test]
    fn test_resource_identifier_renames_to_type() {
        let id = ResourceIdentifier::new("people", "9");
        let json = serde_json::to_value(&id).unwrap();
        // The field MUST serialize as "type", never "resource_type".
        assert_eq!(json["type"], "people");
        assert_eq!(json["id"], "9");
        assert!(json.get("resource_type").is_none());
    }

    #[test]
    fn test_resource_identifier_deserializes_from_type() {
        // Deserialization must also accept the wire name "type".
        let id: ResourceIdentifier =
            serde_json::from_str(r#"{"type":"people","id":"9"}"#).unwrap();
        assert_eq!(id.resource_type, "people");
        assert_eq!(id.id, "9");
    }

    #[test]
    fn test_empty_relationship_map_still_serializes_as_object() {
        // A resource that returns Some(empty map) -> relationships present but {}.
        #[derive(Serialize)]
        struct Empties {
            id: u64,
        }
        impl JsonApiResource for Empties {
            fn json_api_type() -> &'static str {
                "empties"
            }
            fn json_api_id(&self) -> String {
                self.id.to_string()
            }
            fn relationships(&self) -> Option<RelationshipMap> {
                Some(RelationshipMap::new())
            }
        }
        let json = serde_json::to_value(&Empties { id: 1 }.to_document()).unwrap();
        // Some(empty) is still "present" (Option::is_none is false), so it appears
        // as an empty object. This documents current behavior.
        assert!(json["data"]["relationships"].is_object());
        assert_eq!(json["data"]["relationships"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_collection_document_data_is_array() {
        let doc = JsonApiDocument::collection(vec![]);
        let json = serde_json::to_value(&doc).unwrap();
        // Empty collection => data: [] (array), not null/object.
        assert!(json["data"].is_array());
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
        // Empty optional members omitted.
        assert!(json.get("meta").is_none());
        assert!(json.get("links").is_none());
        assert!(json.get("included").is_none());
    }

    #[test]
    fn test_included_omitted_when_empty_present_when_set() {
        let bare = sample().to_document();
        let json = serde_json::to_value(&bare).unwrap();
        assert!(json.get("included").is_none());

        let with = sample().to_document().with_included(vec![ResourceObject {
            resource_type: "people".into(),
            id: "9".into(),
            attributes: Map::new(),
            relationships: None,
            links: None,
        }]);
        let json2 = serde_json::to_value(&with).unwrap();
        assert!(json2["included"].is_array());
        assert_eq!(json2["included"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_single_object_with_no_links_omits_links() {
        #[derive(Serialize)]
        struct NoLink {
            id: u64,
            v: i32,
        }
        impl JsonApiResource for NoLink {
            fn json_api_type() -> &'static str {
                "nolink"
            }
            fn json_api_id(&self) -> String {
                self.id.to_string()
            }
        }
        let json = serde_json::to_value(&NoLink { id: 1, v: 2 }.to_document()).unwrap();
        assert!(json["data"].get("links").is_none());
    }
}
