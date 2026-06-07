//! Integration tests for rf-api-resources
//!
//! Tests cover: Resource::to_json, WrappedResource, ResourceWithMeta,
//! Collection, PaginatedCollection, PaginationMeta, PaginationLinks,
//! and ConditionalAttribute.

use rf_api_resources::{
    collection::{Collection, PaginatedCollection, PaginationLinks, PaginationMeta},
    resource::{ConditionalAttribute, Resource},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

// ───────────────────────────────────────────────────────────────────────────
// Fixture types
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct UserResource {
    id: i64,
    name: String,
    email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
}

impl Resource for UserResource {}

#[derive(Debug, Clone, Serialize)]
struct ProductResource {
    id: i64,
    title: String,
    price_cents: i64,
}

impl Resource for ProductResource {}

fn alice() -> UserResource {
    UserResource {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        role: None,
    }
}

fn bob() -> UserResource {
    UserResource {
        id: 2,
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
        role: Some("admin".to_string()),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Resource::to_json
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn resource_to_json_produces_object() {
    let json = alice().to_json().unwrap();
    assert!(json.is_object());
}

#[test]
fn resource_to_json_contains_correct_id() {
    let json = alice().to_json().unwrap();
    assert_eq!(json["id"], 1);
}

#[test]
fn resource_to_json_contains_correct_name() {
    let json = alice().to_json().unwrap();
    assert_eq!(json["name"], "Alice");
}

#[test]
fn resource_to_json_skips_none_optional_field() {
    let json = alice().to_json().unwrap();
    // role is None and skip_serializing_if = Option::is_none
    assert!(json.get("role").is_none() || json["role"].is_null());
}

#[test]
fn resource_to_json_includes_some_optional_field() {
    let json = bob().to_json().unwrap();
    assert_eq!(json["role"], "admin");
}

// ───────────────────────────────────────────────────────────────────────────
// WrappedResource
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn wrapped_resource_nests_under_custom_key() {
    let wrapped = alice().wrap("data");
    let json = wrapped.to_json().unwrap();
    assert!(json["data"].is_object());
    assert_eq!(json["data"]["id"], 1);
}

#[test]
fn wrapped_resource_different_key() {
    let wrapped = bob().wrap("user");
    let json = wrapped.to_json().unwrap();
    assert!(json["user"].is_object());
    assert_eq!(json["user"]["name"], "Bob");
}

// ───────────────────────────────────────────────────────────────────────────
// ResourceWithMeta
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn resource_with_meta_includes_meta_key() {
    let mut meta = HashMap::new();
    meta.insert("version".to_string(), Value::String("2.0".to_string()));

    let with_meta = alice().with_meta(meta);
    let json = with_meta.to_json().unwrap();

    assert!(json["meta"].is_object());
    assert_eq!(json["meta"]["version"], "2.0");
}

#[test]
fn resource_with_meta_keeps_original_fields() {
    let mut meta = HashMap::new();
    meta.insert("req_id".to_string(), Value::String("abc".to_string()));

    let with_meta = alice().with_meta(meta);
    let json = with_meta.to_json().unwrap();

    assert_eq!(json["id"], 1);
    assert_eq!(json["email"], "alice@example.com");
}

// ───────────────────────────────────────────────────────────────────────────
// Collection
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn collection_count_matches_number_of_items() {
    let c = Collection::new(vec![alice(), bob()]);
    assert_eq!(c.count(), 2);
}

#[test]
fn empty_collection_reports_is_empty() {
    let c: Collection<UserResource> = Collection::new(vec![]);
    assert!(c.is_empty());
}

#[test]
fn collection_items_returns_slice_of_correct_length() {
    use rf_api_resources::collection::ResourceCollection;
    let c = Collection::new(vec![alice(), bob()]);
    assert_eq!(c.items().len(), 2);
}

// ───────────────────────────────────────────────────────────────────────────
// PaginationMeta
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn pagination_meta_last_page_is_ceiling_division() {
    let meta = PaginationMeta::new(1, 10, 25);
    assert_eq!(meta.last_page, 3); // ceil(25/10)
}

#[test]
fn pagination_meta_has_next_on_non_last_page() {
    let meta = PaginationMeta::new(1, 10, 25);
    assert!(meta.has_next_page());
}

#[test]
fn pagination_meta_no_next_on_last_page() {
    let meta = PaginationMeta::new(3, 10, 25);
    assert!(!meta.has_next_page());
}

#[test]
fn pagination_meta_has_previous_on_non_first_page() {
    let meta = PaginationMeta::new(2, 10, 25);
    assert!(meta.has_previous_page());
}

#[test]
fn pagination_meta_no_previous_on_first_page() {
    let meta = PaginationMeta::new(1, 10, 25);
    assert!(!meta.has_previous_page());
}

// ───────────────────────────────────────────────────────────────────────────
// PaginatedCollection
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn paginated_collection_stores_meta() {
    let meta = PaginationMeta::new(1, 10, 3);
    let pc = PaginatedCollection::new(vec![alice()], meta);
    assert_eq!(pc.meta().total, 3);
}

// ───────────────────────────────────────────────────────────────────────────
// PaginationLinks
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn pagination_links_first_always_points_to_page_1() {
    let meta = PaginationMeta::new(2, 10, 50);
    let links = PaginationLinks::new("/api/users", &meta);
    assert_eq!(links.first, "/api/users?page=1");
}

#[test]
fn pagination_links_next_points_to_next_page() {
    let meta = PaginationMeta::new(2, 10, 50);
    let links = PaginationLinks::new("/api/users", &meta);
    assert_eq!(links.next, Some("/api/users?page=3".to_string()));
}

#[test]
fn pagination_links_prev_points_to_previous_page() {
    let meta = PaginationMeta::new(3, 10, 50);
    let links = PaginationLinks::new("/api/users", &meta);
    assert_eq!(links.prev, Some("/api/users?page=2".to_string()));
}

// ───────────────────────────────────────────────────────────────────────────
// ConditionalAttribute
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn when_true_includes_attribute() {
    let secret = "password123";
    assert!(secret.when(true).is_some());
}

#[test]
fn when_false_excludes_attribute() {
    let secret = "password123";
    assert!(secret.when(false).is_none());
}

#[test]
fn unless_true_excludes_attribute() {
    let val = 42;
    assert!(val.unless(true).is_none());
}

#[test]
fn unless_false_includes_attribute() {
    let val = 42;
    assert!(val.unless(false).is_some());
}
