//! Integration tests for rf-collections
//!
//! Tests cover: map, filter, reduce, group_by, sort_by, chunk, first, last,
//! contains, pluck, unique, flatten, zip, sum, avg, min, max, for_page,
//! and empty-collection stability.

use rf_collections::{collect, Collection};

// ───────────────────────────────────────────────────────────────────────────
// Helper struct
// ───────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct User {
    id: i64,
    name: String,
    score: i64,
    active: bool,
}

impl User {
    fn new(id: i64, name: &str, score: i64, active: bool) -> Self {
        Self {
            id,
            name: name.to_string(),
            score,
            active,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// map
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn map_transforms_every_item() {
    let result = collect(vec![1, 2, 3]).map(|x| x * 10).to_vec();
    assert_eq!(result, vec![10, 20, 30]);
}

#[test]
fn map_changes_type() {
    let result = collect(vec!["hello", "world"])
        .map(|s| s.len())
        .to_vec();
    assert_eq!(result, vec![5, 5]);
}

// ───────────────────────────────────────────────────────────────────────────
// filter
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn filter_removes_non_matching_items() {
    let result = collect(vec![1, 2, 3, 4, 5, 6])
        .filter(|x| x % 2 == 0)
        .to_vec();
    assert_eq!(result, vec![2, 4, 6]);
}

#[test]
fn filter_all_removed_returns_empty() {
    let result = collect(vec![1, 3, 5])
        .filter(|x| x % 2 == 0)
        .to_vec();
    assert!(result.is_empty());
}

// ───────────────────────────────────────────────────────────────────────────
// reduce
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn reduce_sums_values() {
    let total = collect(vec![10, 20, 30]).reduce(0, |acc, x| acc + x);
    assert_eq!(total, 60);
}

#[test]
fn reduce_builds_string() {
    let words = collect(vec!["Hello", " ", "World"])
        .reduce(String::new(), |mut acc, s| {
            acc.push_str(s);
            acc
        });
    assert_eq!(words, "Hello World");
}

// ───────────────────────────────────────────────────────────────────────────
// group_by
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn group_by_groups_by_boolean_field() {
    let users = vec![
        User::new(1, "Alice", 90, true),
        User::new(2, "Bob", 70, false),
        User::new(3, "Carol", 85, true),
    ];
    let groups = collect(users).group_by(|u| u.active);
    assert_eq!(groups[&true].len(), 2);
    assert_eq!(groups[&false].len(), 1);
}

#[test]
fn group_by_single_group_when_all_same_key() {
    let numbers = vec![2, 4, 6, 8];
    let groups = collect(numbers).group_by(|n| n % 2);
    assert_eq!(groups[&0].len(), 4);
    assert!(!groups.contains_key(&1));
}

// ───────────────────────────────────────────────────────────────────────────
// sort_by
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn sort_by_ascending_score() {
    let users = vec![
        User::new(1, "Bob", 70, true),
        User::new(2, "Alice", 90, true),
        User::new(3, "Carol", 80, true),
    ];
    let sorted: Vec<String> = collect(users)
        .sort_by(|u| u.score)
        .map(|u| u.name.clone())
        .to_vec();
    assert_eq!(sorted, vec!["Bob", "Carol", "Alice"]);
}

#[test]
fn sort_by_string_key() {
    let words = vec!["banana", "apple", "cherry"];
    let sorted = collect(words).sort_by(|s| *s).to_vec();
    assert_eq!(sorted, vec!["apple", "banana", "cherry"]);
}

// ───────────────────────────────────────────────────────────────────────────
// chunk
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn chunk_splits_into_equal_groups() {
    let chunks = collect(vec![1, 2, 3, 4, 5, 6]).chunk(2);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].all(), &[1, 2]);
    assert_eq!(chunks[1].all(), &[3, 4]);
    assert_eq!(chunks[2].all(), &[5, 6]);
}

#[test]
fn chunk_last_group_may_be_smaller() {
    let chunks = collect(vec![1, 2, 3, 4, 5]).chunk(2);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[2].all(), &[5]);
}

// ───────────────────────────────────────────────────────────────────────────
// first / last
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn first_returns_first_item() {
    let c = collect(vec![10, 20, 30]);
    assert_eq!(c.first(), Some(&10));
}

#[test]
fn last_returns_last_item() {
    let c = collect(vec![10, 20, 30]);
    assert_eq!(c.last(), Some(&30));
}

#[test]
fn first_and_last_are_none_on_empty() {
    let c: Collection<i32> = collect(vec![]);
    assert_eq!(c.first(), None);
    assert_eq!(c.last(), None);
}

// ───────────────────────────────────────────────────────────────────────────
// contains
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn contains_finds_existing_item() {
    let c = collect(vec!["a", "b", "c"]);
    assert!(c.contains(&"b"));
}

#[test]
fn contains_returns_false_for_missing_item() {
    let c = collect(vec!["a", "b", "c"]);
    assert!(!c.contains(&"z"));
}

// ───────────────────────────────────────────────────────────────────────────
// pluck
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn pluck_extracts_single_field() {
    let users = vec![
        User::new(1, "Alice", 90, true),
        User::new(2, "Bob", 70, false),
    ];
    let names = collect(users).pluck(|u| u.name.clone()).to_vec();
    assert_eq!(names, vec!["Alice", "Bob"]);
}

// ───────────────────────────────────────────────────────────────────────────
// unique
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn unique_removes_duplicates_preserving_order() {
    let result = collect(vec![1, 2, 2, 3, 1, 4, 3]).unique().to_vec();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn unique_on_already_unique_collection_unchanged() {
    let result = collect(vec![10, 20, 30]).unique().to_vec();
    assert_eq!(result, vec![10, 20, 30]);
}

// ───────────────────────────────────────────────────────────────────────────
// flatten
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn flatten_nested_vecs_into_single_collection() {
    let nested = vec![vec![1, 2], vec![3, 4], vec![5]];
    let result = collect(nested).flatten().to_vec();
    assert_eq!(result, vec![1, 2, 3, 4, 5]);
}

// ───────────────────────────────────────────────────────────────────────────
// zip
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn zip_combines_two_collections() {
    let keys = vec!["a", "b", "c"];
    let vals = vec![1, 2, 3];
    let pairs = collect(keys).zip(vals).to_vec();
    assert_eq!(pairs, vec![("a", 1), ("b", 2), ("c", 3)]);
}

#[test]
fn zip_truncates_to_shorter_collection() {
    let a = vec![1, 2, 3, 4];
    let b = vec!["x", "y"];
    let result = collect(a).zip(b).to_vec();
    assert_eq!(result.len(), 2);
}

// ───────────────────────────────────────────────────────────────────────────
// sum / avg / min / max
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn sum_adds_all_numeric_values() {
    let total: i32 = collect(vec![1i32, 2, 3, 4, 5]).sum();
    assert_eq!(total, 15);
}

#[test]
fn avg_computes_mean() {
    let mean = collect(vec![10i32, 20, 30]).avg();
    assert_eq!(mean, Some(20.0));
}

#[test]
fn min_returns_smallest() {
    let m = collect(vec![5, 1, 9, 2]).min();
    assert_eq!(m, Some(1));
}

#[test]
fn max_returns_largest() {
    let m = collect(vec![5, 1, 9, 2]).max();
    assert_eq!(m, Some(9));
}

// ───────────────────────────────────────────────────────────────────────────
// for_page (paginate)
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn for_page_returns_correct_items_on_page_1() {
    let items: Vec<i32> = (1..=20).collect();
    let page = collect(items).for_page(1, 5).to_vec();
    assert_eq!(page, vec![1, 2, 3, 4, 5]);
}

#[test]
fn for_page_returns_correct_items_on_page_3() {
    let items: Vec<i32> = (1..=20).collect();
    let page = collect(items).for_page(3, 5).to_vec();
    assert_eq!(page, vec![11, 12, 13, 14, 15]);
}

#[test]
fn for_page_beyond_end_returns_empty() {
    let items: Vec<i32> = (1..=10).collect();
    let page = collect(items).for_page(10, 5).to_vec();
    assert!(page.is_empty());
}

// ───────────────────────────────────────────────────────────────────────────
// Empty collection stability
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn empty_collection_map_returns_empty() {
    let result: Vec<i32> = collect(Vec::<i32>::new()).map(|x| x * 2).to_vec();
    assert!(result.is_empty());
}

#[test]
fn empty_collection_filter_returns_empty() {
    let result: Vec<i32> = collect(Vec::<i32>::new()).filter(|x| *x > 0).to_vec();
    assert!(result.is_empty());
}

#[test]
fn empty_collection_reduce_returns_init() {
    let result = collect(Vec::<i32>::new()).reduce(42, |acc, x| acc + x);
    assert_eq!(result, 42);
}

#[test]
fn empty_collection_sum_is_zero() {
    let result: i32 = collect(Vec::<i32>::new()).sum();
    assert_eq!(result, 0);
}

#[test]
fn empty_collection_avg_is_none() {
    let result = collect(Vec::<i32>::new()).avg();
    assert_eq!(result, None);
}

#[test]
fn empty_collection_min_max_are_none() {
    assert_eq!(collect(Vec::<i32>::new()).min(), None);
    assert_eq!(collect(Vec::<i32>::new()).max(), None);
}

#[test]
fn empty_collection_unique_returns_empty() {
    let result: Vec<i32> = collect(Vec::<i32>::new()).unique().to_vec();
    assert!(result.is_empty());
}

#[test]
fn empty_collection_chunk_returns_empty_vec() {
    let chunks = collect(Vec::<i32>::new()).chunk(5);
    assert!(chunks.is_empty());
}
