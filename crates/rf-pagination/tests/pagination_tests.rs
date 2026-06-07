//! Integration tests for rf-pagination
//!
//! Tests cover: offset paginator (correct items on page X, total / page
//! count, metadata), cursor paginator, has_next / has_prev flags,
//! and pagination links.

use rf_pagination::{
    CursorDirection, CursorPaginator, PaginatedResponse, Paginator, PaginationLinks,
    PaginationMeta,
};

// ───────────────────────────────────────────────────────────────────────────
// Offset paginator – basic construction
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn paginator_calculates_correct_last_page() {
    let p = Paginator::new(100, 10, 1).unwrap();
    assert_eq!(p.last_page, 10);
}

#[test]
fn paginator_total_count_matches_input() {
    let p = Paginator::new(57, 10, 1).unwrap();
    assert_eq!(p.total, 57);
    // ceil(57/10) = 6
    assert_eq!(p.last_page, 6);
}

// ───────────────────────────────────────────────────────────────────────────
// Offset / limit for SQL
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn offset_is_zero_on_first_page() {
    let p = Paginator::new(100, 15, 1).unwrap();
    assert_eq!(p.offset(), 0);
}

#[test]
fn offset_is_correct_on_third_page() {
    let p = Paginator::new(100, 10, 3).unwrap();
    // (3-1) * 10 = 20
    assert_eq!(p.offset(), 20);
}

#[test]
fn limit_equals_per_page() {
    let p = Paginator::new(100, 25, 1).unwrap();
    assert_eq!(p.limit(), 25);
}

// ───────────────────────────────────────────────────────────────────────────
// has_next / has_prev
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn first_page_has_next_but_not_prev() {
    let p = Paginator::new(100, 10, 1).unwrap();
    assert!(p.has_next());
    assert!(!p.has_prev());
}

#[test]
fn last_page_has_prev_but_not_next() {
    let p = Paginator::new(100, 10, 10).unwrap();
    assert!(!p.has_next());
    assert!(p.has_prev());
}

#[test]
fn middle_page_has_both_next_and_prev() {
    let p = Paginator::new(100, 10, 5).unwrap();
    assert!(p.has_next());
    assert!(p.has_prev());
}

// ───────────────────────────────────────────────────────────────────────────
// from / to item numbers
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn from_is_one_on_first_page() {
    let p = Paginator::new(100, 10, 1).unwrap();
    assert_eq!(p.from(), 1);
    assert_eq!(p.to(), 10);
}

#[test]
fn from_to_correct_on_middle_page() {
    let p = Paginator::new(100, 10, 3).unwrap();
    assert_eq!(p.from(), 21);
    assert_eq!(p.to(), 30);
}

#[test]
fn to_is_clamped_to_total_on_last_partial_page() {
    // 23 items, 10 per page → page 3 has items 21-23
    let p = Paginator::new(23, 10, 3).unwrap();
    assert_eq!(p.from(), 21);
    assert_eq!(p.to(), 23);
}

// ───────────────────────────────────────────────────────────────────────────
// Metadata
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn pagination_meta_has_correct_current_page() {
    let p = Paginator::new(100, 10, 4).unwrap();
    let meta = PaginationMeta::from(p);
    assert_eq!(meta.current_page, 4);
    assert_eq!(meta.from, 31);
    assert_eq!(meta.to, 40);
}

// ───────────────────────────────────────────────────────────────────────────
// Pagination links
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn links_contains_first_and_last() {
    let p = Paginator::new(100, 10, 3).unwrap();
    let links = PaginationLinks::new("/api/items", &p);
    assert_eq!(links.first, Some("/api/items?page=1".to_string()));
    assert_eq!(links.last, Some("/api/items?page=10".to_string()));
}

#[test]
fn links_prev_and_next_correct_for_middle_page() {
    let p = Paginator::new(100, 10, 5).unwrap();
    let links = PaginationLinks::new("/api/items", &p);
    assert_eq!(links.prev, Some("/api/items?page=4".to_string()));
    assert_eq!(links.next, Some("/api/items?page=6".to_string()));
}

#[test]
fn links_prev_is_none_on_first_page() {
    let p = Paginator::new(100, 10, 1).unwrap();
    let links = PaginationLinks::new("/api/items", &p);
    assert_eq!(links.prev, None);
}

// ───────────────────────────────────────────────────────────────────────────
// PaginatedResponse wraps data + meta
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn paginated_response_has_correct_data_and_meta() {
    let p = Paginator::new(3, 10, 1).unwrap();
    let resp = PaginatedResponse::new(vec!["a", "b", "c"], p, None);
    assert_eq!(resp.data.len(), 3);
    assert_eq!(resp.meta.total, 3);
    assert_eq!(resp.meta.current_page, 1);
    assert!(resp.links.is_none());
}

#[test]
fn paginated_response_includes_links_when_base_url_given() {
    let p = Paginator::new(50, 10, 2).unwrap();
    let resp = PaginatedResponse::new(vec![1, 2, 3], p, Some("/api/v1/users"));
    assert!(resp.links.is_some());
    let links = resp.links.unwrap();
    assert!(links.next.is_some());
    assert!(links.prev.is_some());
}

// ───────────────────────────────────────────────────────────────────────────
// Cursor paginator
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn cursor_paginator_has_no_cursor_initially() {
    let cp = CursorPaginator::new(25).unwrap();
    assert!(cp.cursor.is_none());
}

#[test]
fn cursor_paginator_after_sets_direction_after() {
    let cp = CursorPaginator::new(25).unwrap().after("abc123".to_string());
    let cursor = cp.cursor.unwrap();
    assert_eq!(cursor.value, "abc123");
    assert_eq!(cursor.direction, CursorDirection::After);
}

#[test]
fn cursor_paginator_before_sets_direction_before() {
    let cp = CursorPaginator::new(10).unwrap().before("xyz".to_string());
    let cursor = cp.cursor.unwrap();
    assert_eq!(cursor.direction, CursorDirection::Before);
}

#[test]
fn cursor_paginator_limit_equals_per_page() {
    let cp = CursorPaginator::new(30).unwrap();
    assert_eq!(cp.limit(), 30);
}

// ───────────────────────────────────────────────────────────────────────────
// Error cases
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn paginator_errors_on_zero_per_page() {
    assert!(Paginator::new(100, 0, 1).is_err());
}

#[test]
fn paginator_errors_on_negative_per_page() {
    assert!(Paginator::new(100, -1, 1).is_err());
}

#[test]
fn paginator_errors_on_zero_page() {
    assert!(Paginator::new(100, 10, 0).is_err());
}

#[test]
fn cursor_paginator_errors_on_zero_per_page() {
    assert!(CursorPaginator::new(0).is_err());
}
