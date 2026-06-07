//! Integration tests for rf-helpers
//!
//! Tests cover: str helpers (snake_case, camel_case, slug, plural, singular,
//! limit, before/after), arr helpers (flatten, pluck-style first/last,
//! contains, group_by), and url helpers (url, asset, encode/decode).

use rf_helpers::{arr, str, url};
use std::collections::HashMap;

// ───────────────────────────────────────────────────────────────────────────
// String helpers
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn snake_case_converts_pascal_case() {
    assert_eq!(str::snake("HelloWorld"), "hello_world");
}

#[test]
fn snake_case_converts_camel_case() {
    assert_eq!(str::snake("helloWorld"), "hello_world");
}

#[test]
fn camel_case_converts_snake_case() {
    assert_eq!(str::camel("hello_world"), "helloWorld");
}

#[test]
fn camel_case_lowercases_first_character_of_pascal() {
    assert_eq!(str::camel("HelloWorld"), "helloWorld");
}

#[test]
fn slug_converts_spaces_to_hyphens() {
    assert_eq!(str::slug("Hello World"), "hello-world");
}

#[test]
fn slug_strips_accents_from_unicode() {
    assert_eq!(str::slug("café au lait"), "cafe-au-lait");
}

#[test]
fn slug_collapses_multiple_separators() {
    assert_eq!(str::slug("Hello  World"), "hello-world");
}

#[test]
fn plural_adds_s_to_regular_nouns() {
    assert_eq!(str::plural("user"), "users");
}

#[test]
fn plural_handles_irregular_person() {
    assert_eq!(str::plural("person"), "people");
}

#[test]
fn singular_removes_trailing_s() {
    assert_eq!(str::singular("users"), "user");
}

#[test]
fn singular_handles_irregular_people() {
    assert_eq!(str::singular("people"), "person");
}

#[test]
fn limit_truncates_long_strings() {
    assert_eq!(str::limit("Hello World", 5, "..."), "Hello...");
}

#[test]
fn limit_does_not_truncate_short_strings() {
    assert_eq!(str::limit("Hi", 10, "..."), "Hi");
}

#[test]
fn before_returns_portion_before_delimiter() {
    assert_eq!(str::before("user@example.com", "@"), "user");
}

#[test]
fn after_returns_portion_after_delimiter() {
    assert_eq!(str::after("user@example.com", "@"), "example.com");
}

// ───────────────────────────────────────────────────────────────────────────
// Array helpers
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn arr_flatten_merges_nested_vecs() {
    let nested = vec![vec![1, 2], vec![3, 4], vec![5]];
    let result = arr::flatten(nested);
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
}

#[test]
fn arr_first_finds_matching_element() {
    let items = vec![10, 20, 30, 40];
    let found = arr::first(&items, |&x| x > 15);
    assert_eq!(found, Some(&20));
}

#[test]
fn arr_first_returns_none_when_no_match() {
    let items = vec![1, 2, 3];
    let found = arr::first(&items, |&x| x > 100);
    assert_eq!(found, None);
}

#[test]
fn arr_last_finds_last_matching_element() {
    let items = vec![10, 20, 30, 40];
    let found = arr::last(&items, |&x| x < 35);
    assert_eq!(found, Some(&30));
}

#[test]
fn arr_contains_returns_true_for_present_value() {
    let items = vec![1, 2, 3, 4, 5];
    assert!(arr::contains(&items, &3));
}

#[test]
fn arr_contains_returns_false_for_absent_value() {
    let items = vec![1, 2, 3];
    assert!(!arr::contains(&items, &99));
}

#[test]
fn arr_group_by_splits_into_even_and_odd() {
    let items = vec![1, 2, 3, 4, 5, 6];
    let groups = arr::group_by(items, |x| x % 2);
    assert_eq!(groups[&0], vec![2, 4, 6]);
    assert_eq!(groups[&1], vec![1, 3, 5]);
}

// ───────────────────────────────────────────────────────────────────────────
// URL helpers
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn url_helper_prepends_base_url() {
    assert_eq!(
        url::url("/users", Some("http://example.com")),
        "http://example.com/users"
    );
}

#[test]
fn url_helper_adds_leading_slash_when_missing() {
    let result = url::url("users", Some("http://example.com"));
    assert_eq!(result, "http://example.com/users");
}

#[test]
fn asset_helper_prepends_assets_prefix() {
    let result = url::asset("css/app.css", None);
    assert!(result.ends_with("/assets/css/app.css"));
}

#[test]
fn encode_percent_encodes_spaces() {
    assert_eq!(url::encode("hello world"), "hello%20world");
}

#[test]
fn decode_reverses_percent_encoding() {
    assert_eq!(url::decode("hello%20world").unwrap(), "hello world");
}
