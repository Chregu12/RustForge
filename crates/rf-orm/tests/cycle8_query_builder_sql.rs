//! Cycle-8 SQL generation tests for `QueryBuilderFacade`.
//!
//! These tests assert the EXACT SQL string and bound-value list produced by
//! `build_sql()` / `build_select_sql()` for all supported clause types.
//! Every test is self-contained (no DB connection needed); the assertions will
//! fail if the query-builder ever silently changes the generated SQL.

use rf_orm::QueryBuilderFacade as QB;
use serde_json::{json, Value};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Build SQL from the builder and split into (sql_string, bindings).
fn sql(b: QB) -> (String, Vec<Value>) {
    b.build_sql()
}

// ── bare SELECT ───────────────────────────────────────────────────────────────

#[test]
fn test_sql_bare_table() {
    let (s, b) = sql(QB::new("users"));
    assert_eq!(s, "SELECT * FROM users");
    assert!(b.is_empty());
}

#[test]
fn test_sql_select_columns() {
    let (s, b) = sql(QB::new("users").select(&["id", "name", "email"]));
    assert_eq!(s, "SELECT id, name, email FROM users");
    assert!(b.is_empty());
}

#[test]
fn test_sql_select_distinct() {
    let (s, _) = sql(QB::new("orders").select(&["status"]).distinct());
    assert_eq!(s, "SELECT DISTINCT status FROM orders");
}

// ── WHERE equality ───────────────────────────────────────────────────────────

#[test]
fn test_sql_where_eq_bool() {
    let (s, b) = sql(QB::new("users").filter("active", true));
    assert_eq!(s, "SELECT * FROM users WHERE active = ?");
    assert_eq!(b, vec![json!(true)]);
}

#[test]
fn test_sql_where_eq_string() {
    let (s, b) = sql(QB::new("posts").filter("status", "published"));
    assert_eq!(s, "SELECT * FROM posts WHERE status = ?");
    assert_eq!(b, vec![json!("published")]);
}

#[test]
fn test_sql_where_eq_integer() {
    let (s, b) = sql(QB::new("comments").filter("post_id", 42i64));
    assert_eq!(s, "SELECT * FROM comments WHERE post_id = ?");
    assert_eq!(b, vec![json!(42i64)]);
}

// ── WHERE with operators ─────────────────────────────────────────────────────

#[test]
fn test_sql_where_op_gte() {
    let (s, b) = sql(QB::new("users").where_op("age", ">=", 18i64));
    assert_eq!(s, "SELECT * FROM users WHERE age >= ?");
    assert_eq!(b, vec![json!(18i64)]);
}

#[test]
fn test_sql_where_op_lt() {
    let (s, b) = sql(QB::new("products").where_op("price", "<", 100i64));
    assert_eq!(s, "SELECT * FROM products WHERE price < ?");
    assert_eq!(b, vec![json!(100i64)]);
}

#[test]
fn test_sql_where_ne() {
    let (s, b) = sql(QB::new("users").where_op("role", "!=", "guest"));
    assert_eq!(s, "SELECT * FROM users WHERE role != ?");
    assert_eq!(b, vec![json!("guest")]);
}

// ── Multiple AND WHERE ───────────────────────────────────────────────────────

#[test]
fn test_sql_multiple_and_wheres() {
    let (s, b) = sql(
        QB::new("users")
            .filter("active", true)
            .where_op("age", ">=", 18i64),
    );
    assert_eq!(s, "SELECT * FROM users WHERE active = ? AND age >= ?");
    assert_eq!(b, vec![json!(true), json!(18i64)]);
}

#[test]
fn test_sql_three_and_wheres() {
    let (s, b) = sql(
        QB::new("posts")
            .filter("published", true)
            .filter("featured", true)
            .where_op("views", ">", 100i64),
    );
    assert_eq!(
        s,
        "SELECT * FROM posts WHERE published = ? AND featured = ? AND views > ?"
    );
    assert_eq!(b.len(), 3);
}

// ── NULL checks ──────────────────────────────────────────────────────────────

#[test]
fn test_sql_where_null() {
    let (s, b) = sql(QB::new("users").where_null("deleted_at"));
    assert_eq!(s, "SELECT * FROM users WHERE deleted_at IS NULL");
    assert!(b.is_empty());
}

#[test]
fn test_sql_where_not_null() {
    let (s, b) = sql(QB::new("users").where_not_null("verified_at"));
    assert_eq!(s, "SELECT * FROM users WHERE verified_at IS NOT NULL");
    assert!(b.is_empty());
}

// ── OR WHERE ─────────────────────────────────────────────────────────────────

#[test]
fn test_sql_or_where_standalone() {
    // With no AND conditions the OR becomes the first (and only) clause.
    let (s, b) = sql(QB::new("users").orWhere("role", "admin"));
    assert_eq!(s, "SELECT * FROM users WHERE role = ?");
    assert_eq!(b, vec![json!("admin")]);
}

#[test]
fn test_sql_and_then_or_where() {
    let (s, b) = sql(
        QB::new("users")
            .filter("role", "admin")
            .orWhere("role", "moderator"),
    );
    assert_eq!(s, "SELECT * FROM users WHERE role = ? OR role = ?");
    assert_eq!(b, vec![json!("admin"), json!("moderator")]);
}

// ── LIKE ─────────────────────────────────────────────────────────────────────

#[test]
fn test_sql_where_like() {
    let (s, b) = sql(QB::new("users").where_like("name", "%John%"));
    assert_eq!(s, "SELECT * FROM users WHERE name LIKE ?");
    assert_eq!(b, vec![json!("%John%")]);
}

#[test]
fn test_sql_where_not_like() {
    let (s, b) = sql(QB::new("users").whereNotLike("email", "%@spam.com"));
    assert_eq!(s, "SELECT * FROM users WHERE email NOT LIKE ?");
    assert_eq!(b, vec![json!("%@spam.com")]);
}

// ── IN / NOT IN ──────────────────────────────────────────────────────────────

#[test]
fn test_sql_where_in_expands_to_individual_placeholders() {
    // Bug before fix: emitted `status IN ?` with the whole array as one binding.
    let (s, b) = sql(QB::new("posts").where_in("status", vec!["draft", "published"]));
    assert_eq!(s, "SELECT * FROM posts WHERE status IN (?, ?)");
    assert_eq!(b, vec![json!("draft"), json!("published")]);
}

#[test]
fn test_sql_where_not_in_expands_to_individual_placeholders() {
    let (s, b) = sql(QB::new("orders").where_not_in("status", vec!["cancelled", "refunded"]));
    assert_eq!(s, "SELECT * FROM orders WHERE status NOT IN (?, ?)");
    assert_eq!(b, vec![json!("cancelled"), json!("refunded")]);
}

#[test]
fn test_sql_where_in_three_values() {
    let (s, b) = sql(QB::new("users").where_in("id", vec![1i64, 2i64, 3i64]));
    assert_eq!(s, "SELECT * FROM users WHERE id IN (?, ?, ?)");
    assert_eq!(b.len(), 3);
    assert_eq!(b[0], json!(1i64));
    assert_eq!(b[2], json!(3i64));
}

#[test]
fn test_sql_where_in_empty_produces_always_false() {
    // An empty IN list must not match any row — not generate invalid SQL.
    let (s, b) = sql(QB::new("users").where_in("id", Vec::<i64>::new()));
    assert_eq!(s, "SELECT * FROM users WHERE 1 = 0");
    assert!(b.is_empty());
}

#[test]
fn test_sql_where_not_in_empty_produces_always_true() {
    let (s, b) = sql(QB::new("users").where_not_in("id", Vec::<i64>::new()));
    assert_eq!(s, "SELECT * FROM users WHERE 1 = 1");
    assert!(b.is_empty());
}

// ── BETWEEN / NOT BETWEEN ─────────────────────────────────────────────────────

#[test]
fn test_sql_where_between_expands_to_gte_lte() {
    // where_between uses two separate AND conditions: col >= min AND col <= max.
    let (s, b) = sql(QB::new("orders").where_between("total", 100i64, 500i64));
    assert_eq!(s, "SELECT * FROM orders WHERE total >= ? AND total <= ?");
    assert_eq!(b, vec![json!(100i64), json!(500i64)]);
}

#[test]
fn test_sql_where_not_between_is_atomic_not_between() {
    // Bug before fix: split across wheres/or_wheres → wrong SQL precedence.
    let (s, b) = sql(QB::new("users").whereNotBetween("age", 18i64, 65i64));
    assert_eq!(s, "SELECT * FROM users WHERE age NOT BETWEEN ? AND ?");
    assert_eq!(b, vec![json!(18i64), json!(65i64)]);
}

#[test]
fn test_sql_where_not_between_chained_with_and_does_not_produce_wrong_or() {
    // Before the fix:
    //   WHERE active = ? AND age < ? OR age > ?
    // After the fix (atomic NOT BETWEEN):
    //   WHERE active = ? AND age NOT BETWEEN ? AND ?
    let (s, b) = sql(
        QB::new("users")
            .filter("active", true)
            .whereNotBetween("age", 18i64, 65i64),
    );
    assert_eq!(
        s,
        "SELECT * FROM users WHERE active = ? AND age NOT BETWEEN ? AND ?"
    );
    assert_eq!(b, vec![json!(true), json!(18i64), json!(65i64)]);
    // The OR variant would incorrectly contain "OR" in the WHERE clause.
    assert!(!s.contains("OR"), "NOT BETWEEN must not emit a stray OR: {s}");
}

// ── ORDER BY ─────────────────────────────────────────────────────────────────

#[test]
fn test_sql_order_by_asc() {
    let (s, _) = sql(QB::new("users").order_by("name", "ASC"));
    assert_eq!(s, "SELECT * FROM users ORDER BY name ASC");
}

#[test]
fn test_sql_order_by_desc() {
    let (s, _) = sql(QB::new("users").order_by_desc("created_at"));
    assert_eq!(s, "SELECT * FROM users ORDER BY created_at DESC");
}

#[test]
fn test_sql_multiple_order_by() {
    let (s, _) = sql(
        QB::new("posts")
            .order_by("published_at", "DESC")
            .order_by_asc("id"),
    );
    assert_eq!(s, "SELECT * FROM posts ORDER BY published_at DESC, id ASC");
}

// ── LIMIT / OFFSET ───────────────────────────────────────────────────────────

#[test]
fn test_sql_limit_only() {
    let (s, _) = sql(QB::new("users").limit(10));
    assert_eq!(s, "SELECT * FROM users LIMIT 10");
}

#[test]
fn test_sql_offset_only() {
    let (s, _) = sql(QB::new("users").offset(20));
    assert_eq!(s, "SELECT * FROM users OFFSET 20");
}

#[test]
fn test_sql_limit_and_offset() {
    let (s, _) = sql(QB::new("users").limit(10).offset(20));
    assert_eq!(s, "SELECT * FROM users LIMIT 10 OFFSET 20");
}

#[test]
fn test_sql_skip_and_take_aliases() {
    let (s, _) = sql(QB::new("users").skip(5).take(10));
    assert_eq!(s, "SELECT * FROM users LIMIT 10 OFFSET 5");
}

// ── GROUP BY ─────────────────────────────────────────────────────────────────

#[test]
fn test_sql_group_by() {
    let (s, _) = sql(QB::new("orders").groupBy("status"));
    assert_eq!(s, "SELECT * FROM orders GROUP BY status");
}

// ── chained complex query ────────────────────────────────────────────────────

#[test]
fn test_sql_complex_chained_query() {
    let (s, b) = sql(
        QB::new("posts")
            .select(&["id", "title", "views"])
            .filter("published", true)
            .where_op("views", ">=", 100i64)
            .order_by_desc("published_at")
            .limit(15)
            .offset(0),
    );
    assert_eq!(
        s,
        "SELECT id, title, views FROM posts WHERE published = ? AND views >= ? ORDER BY published_at DESC LIMIT 15 OFFSET 0"
    );
    assert_eq!(b.len(), 2);
    assert_eq!(b[0], json!(true));
    assert_eq!(b[1], json!(100i64));
}

// ── build_sql is idempotent (builder is not consumed) ────────────────────────

#[test]
fn test_sql_build_sql_is_idempotent() {
    let qb = QB::new("users").filter("active", true).limit(5);
    let (s1, b1) = qb.build_sql();
    let (s2, b2) = qb.build_sql();
    assert_eq!(s1, s2);
    assert_eq!(b1, b2);
}
