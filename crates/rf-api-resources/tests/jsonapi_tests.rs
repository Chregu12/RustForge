use rf_api_resources::{
    JsonApiData, JsonApiDocument, JsonApiLinks, JsonApiRelationship, JsonApiRelationshipData,
    JsonApiResource, JsonApiResourceIdentifier, JsonApiResponseBuilder, JsonApiSerializable,
    JsonApiVersion, SparseFieldset,
};
use serde_json::{json, Value};

// ---------- helpers ----------

fn make_user_resource() -> JsonApiResource {
    JsonApiResource::new(
        "1",
        "users",
        json!({ "name": "Alice", "email": "alice@example.com", "age": 30 }),
    )
}

fn make_article_resource() -> JsonApiResource {
    JsonApiResource::new(
        "42",
        "articles",
        json!({ "title": "Hello World", "body": "Lorem ipsum" }),
    )
}

// ---------- JsonApiResource ----------

#[test]
fn test_resource_fields_set_correctly() {
    let r = make_user_resource();
    assert_eq!(r.id, "1");
    assert_eq!(r.resource_type, "users");
    assert_eq!(r.attributes["name"], "Alice");
}

#[test]
fn test_resource_relationships_empty_by_default() {
    let r = make_user_resource();
    assert!(r.relationships.is_empty());
}

#[test]
fn test_resource_links_none_by_default() {
    let r = make_user_resource();
    assert!(r.links.is_none());
}

#[test]
fn test_resource_meta_none_by_default() {
    let r = make_user_resource();
    assert!(r.meta.is_none());
}

#[test]
fn test_resource_with_meta() {
    let r = make_user_resource().with_meta(json!({ "version": 2 }));
    assert!(r.meta.is_some());
    assert_eq!(r.meta.unwrap()["version"], 2);
}

// ---------- JSON serialization format ----------

#[test]
fn test_resource_serializes_type_and_id() {
    let r = make_user_resource();
    let v: Value = serde_json::to_value(&r).unwrap();
    assert_eq!(v["type"], "users");
    assert_eq!(v["id"], "1");
}

#[test]
fn test_resource_attributes_in_json() {
    let r = make_user_resource();
    let v: Value = serde_json::to_value(&r).unwrap();
    assert_eq!(v["attributes"]["name"], "Alice");
    assert_eq!(v["attributes"]["email"], "alice@example.com");
}

#[test]
fn test_relationships_absent_when_empty() {
    let r = make_user_resource();
    let v: Value = serde_json::to_value(&r).unwrap();
    // relationships should be skipped when empty
    assert!(v.get("relationships").is_none());
}

// ---------- JsonApiDocument ----------

#[test]
fn test_document_single_data() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert_eq!(v["data"]["type"], "users");
    assert_eq!(v["data"]["id"], "1");
}

#[test]
fn test_document_collection_data() {
    let resources = vec![make_user_resource(), make_article_resource()];
    let doc = JsonApiResponseBuilder::new()
        .collection(resources)
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert!(v["data"].is_array());
    assert_eq!(v["data"].as_array().unwrap().len(), 2);
}

#[test]
fn test_document_jsonapi_version_is_1_0() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .build();
    assert_eq!(doc.jsonapi.version, "1.0");
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert_eq!(v["jsonapi"]["version"], "1.0");
}

#[test]
fn test_document_meta_included() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .meta(json!({ "total": 100 }))
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert_eq!(v["meta"]["total"], 100);
}

#[test]
fn test_document_included_resources() {
    let author = JsonApiResource::new("7", "users", json!({ "name": "Bob" }));
    let doc = JsonApiResponseBuilder::new()
        .data(make_article_resource())
        .include(author)
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert!(v["included"].is_array());
    assert_eq!(v["included"][0]["type"], "users");
    assert_eq!(v["included"][0]["id"], "7");
}

#[test]
fn test_document_no_included_omitted() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    // included should be absent when empty
    assert!(v.get("included").is_none());
}

// ---------- Relationships ----------

#[test]
fn test_to_one_relationship() {
    let rel = JsonApiRelationship::to_one("5", "authors");
    let r = make_article_resource().with_relationship("author", rel);
    let v: Value = serde_json::to_value(&r).unwrap();
    assert_eq!(v["relationships"]["author"]["data"]["id"], "5");
    assert_eq!(v["relationships"]["author"]["data"]["type"], "authors");
}

#[test]
fn test_to_many_relationship() {
    let ids = vec![
        JsonApiResourceIdentifier::new("1", "tags"),
        JsonApiResourceIdentifier::new("2", "tags"),
    ];
    let rel = JsonApiRelationship::to_many(ids);
    let r = make_article_resource().with_relationship("tags", rel);
    let v: Value = serde_json::to_value(&r).unwrap();
    let data = v["relationships"]["tags"]["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);
    assert_eq!(data[0]["id"], "1");
}

// ---------- SparseFieldset ----------

#[test]
fn test_sparse_fieldset_filters_attributes() {
    let fieldset = SparseFieldset::new().add("users", vec!["name"]);
    let mut r = make_user_resource();
    fieldset.filter_resource(&mut r);
    let v: Value = serde_json::to_value(&r).unwrap();
    assert!(v["attributes"].get("name").is_some());
    assert!(v["attributes"].get("email").is_none());
    assert!(v["attributes"].get("age").is_none());
}

#[test]
fn test_sparse_fieldset_keeps_all_when_no_rule() {
    let fieldset = SparseFieldset::new(); // no rule for "users"
    let mut r = make_user_resource();
    fieldset.filter_resource(&mut r);
    let v: Value = serde_json::to_value(&r).unwrap();
    assert!(v["attributes"].get("name").is_some());
    assert!(v["attributes"].get("email").is_some());
}

#[test]
fn test_sparse_fieldset_multiple_fields() {
    let fieldset = SparseFieldset::new().add("users", vec!["name", "email"]);
    let mut r = make_user_resource();
    fieldset.filter_resource(&mut r);
    let v: Value = serde_json::to_value(&r).unwrap();
    assert!(v["attributes"].get("name").is_some());
    assert!(v["attributes"].get("email").is_some());
    assert!(v["attributes"].get("age").is_none());
}

// ---------- Links (pagination) ----------

#[test]
fn test_links_pagination_urls() {
    let links = JsonApiLinks::new()
        .first("https://api.example.com/articles?page=1")
        .last("https://api.example.com/articles?page=10")
        .prev("https://api.example.com/articles?page=2")
        .next("https://api.example.com/articles?page=4")
        .self_link("https://api.example.com/articles?page=3");

    let doc = JsonApiResponseBuilder::new()
        .collection(vec![make_article_resource()])
        .links(links)
        .build();

    let v: Value = serde_json::to_value(&doc).unwrap();
    assert_eq!(v["links"]["first"], "https://api.example.com/articles?page=1");
    assert_eq!(v["links"]["last"], "https://api.example.com/articles?page=10");
    assert_eq!(v["links"]["prev"], "https://api.example.com/articles?page=2");
    assert_eq!(v["links"]["next"], "https://api.example.com/articles?page=4");
}

#[test]
fn test_links_absent_when_none() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .build();
    let v: Value = serde_json::to_value(&doc).unwrap();
    assert!(v.get("links").is_none());
}

// ---------- Round-trip serialization ----------

#[test]
fn test_round_trip_single_resource() {
    let original = make_user_resource();
    let json_str = serde_json::to_string(&original).unwrap();
    let restored: JsonApiResource = serde_json::from_str(&json_str).unwrap();
    assert_eq!(original.id, restored.id);
    assert_eq!(original.resource_type, restored.resource_type);
    assert_eq!(original.attributes, restored.attributes);
}

#[test]
fn test_round_trip_document() {
    let doc = JsonApiResponseBuilder::new()
        .data(make_user_resource())
        .meta(json!({ "count": 1 }))
        .build();
    let json_str = serde_json::to_string(&doc).unwrap();
    let restored: JsonApiDocument = serde_json::from_str(&json_str).unwrap();
    assert_eq!(restored.jsonapi.version, "1.0");
    if let JsonApiData::Single(res) = restored.data {
        assert_eq!(res.id, "1");
    } else {
        panic!("expected Single data");
    }
}

// ---------- JsonApiSerializable trait ----------

struct User {
    id: u64,
    name: String,
    email: String,
}

impl JsonApiSerializable for User {
    fn resource_type() -> &'static str {
        "users"
    }

    fn to_jsonapi(&self) -> JsonApiResource {
        JsonApiResource::new(
            self.id.to_string(),
            Self::resource_type(),
            json!({ "name": self.name, "email": self.email }),
        )
    }
}

#[test]
fn test_jsonapi_serializable_trait() {
    let user = User {
        id: 99,
        name: "Carol".to_string(),
        email: "carol@example.com".to_string(),
    };
    let resource = user.to_jsonapi();
    assert_eq!(resource.id, "99");
    assert_eq!(resource.resource_type, "users");
    assert_eq!(resource.attributes["name"], "Carol");
    assert_eq!(User::resource_type(), "users");
}

// ---------- JsonApiVersion ----------

#[test]
fn test_jsonapi_version_default_is_1_0() {
    let v = JsonApiVersion::default();
    assert_eq!(v.version, "1.0");
}

#[test]
fn test_jsonapi_version_v1_constructor() {
    let v = JsonApiVersion::v1();
    assert_eq!(v.version, "1.0");
}
