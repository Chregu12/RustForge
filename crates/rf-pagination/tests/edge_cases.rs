//! Boundary-condition tests for pagination math.
//!
//! The inline unit tests cover the happy path; these pin down the edges that
//! are easy to get wrong: empty result sets, partial last pages, exactly-full
//! pages, single-page sets, and pages requested past the end.

use rf_pagination::{Paginator, PaginationError};

#[test]
fn empty_result_set_has_no_items_and_no_neighbours() {
    let p = Paginator::new(0, 15, 1).unwrap();
    assert_eq!(p.last_page, 0);
    assert_eq!(p.from(), 0, "from() is 0 when there is nothing to show");
    assert_eq!(p.to(), 0);
    assert!(!p.has_next());
    assert!(!p.has_prev());
    assert_eq!(p.next_page(), None);
    assert_eq!(p.prev_page(), None);
    assert_eq!(p.offset(), 0);
}

#[test]
fn partial_last_page_clamps_to_total() {
    // 25 items, 10 per page -> 3 pages, last page holds items 21..=25.
    let p = Paginator::new(25, 10, 3).unwrap();
    assert_eq!(p.last_page, 3);
    assert_eq!(p.offset(), 20);
    assert_eq!(p.from(), 21);
    assert_eq!(p.to(), 25, "to() is clamped to total, not offset + per_page");
    assert!(!p.has_next());
    assert!(p.has_prev());
    assert_eq!(p.prev_page(), Some(2));
}

#[test]
fn exactly_full_last_page() {
    // 30 items, 10 per page -> exactly 3 full pages.
    let p = Paginator::new(30, 10, 3).unwrap();
    assert_eq!(p.last_page, 3);
    assert_eq!(p.from(), 21);
    assert_eq!(p.to(), 30);
    assert!(!p.has_next());
}

#[test]
fn middle_page_has_both_neighbours() {
    let p = Paginator::new(100, 10, 5).unwrap();
    assert_eq!(p.last_page, 10);
    assert_eq!(p.offset(), 40);
    assert_eq!(p.from(), 41);
    assert_eq!(p.to(), 50);
    assert!(p.has_next());
    assert!(p.has_prev());
    assert_eq!(p.next_page(), Some(6));
    assert_eq!(p.prev_page(), Some(4));
}

#[test]
fn single_page_when_total_fits_in_one() {
    let p = Paginator::new(5, 10, 1).unwrap();
    assert_eq!(p.last_page, 1);
    assert_eq!(p.from(), 1);
    assert_eq!(p.to(), 5);
    assert!(!p.has_next());
    assert!(!p.has_prev());
}

#[test]
fn ceil_rounds_a_remainder_up_to_an_extra_page() {
    // 21 items, 10 per page -> 3 pages (not 2).
    let p = Paginator::new(21, 10, 1).unwrap();
    assert_eq!(p.last_page, 3);
}

#[test]
fn page_requested_past_the_end_yields_empty_range() {
    // Page 9 of a 3-page set: offset runs past total, so from() > to().
    let p = Paginator::new(25, 10, 9).unwrap();
    assert_eq!(p.offset(), 80);
    assert_eq!(p.to(), 25, "to() never exceeds total");
    assert!(!p.has_next());
}

#[test]
fn invalid_per_page_and_page_are_rejected() {
    assert!(matches!(
        Paginator::new(10, 0, 1),
        Err(PaginationError::InvalidPerPage(0))
    ));
    assert!(matches!(
        Paginator::new(10, -5, 1),
        Err(PaginationError::InvalidPerPage(-5))
    ));
    assert!(matches!(
        Paginator::new(10, 10, 0),
        Err(PaginationError::InvalidPage(0))
    ));
    assert!(matches!(
        Paginator::new(10, 10, -1),
        Err(PaginationError::InvalidPage(-1))
    ));
}
