//! Cycle-8 CRUD round-trip tests for the SQLite-backed `DB` / `QueryBuilderFacade`.
//!
//! Each test uses a uniquely-named table to avoid collisions with the shared
//! `GLOBAL_DB` singleton.  Tables are (re-)created + cleared at the start of
//! each test so tests are self-contained and order-independent.

use rf_orm::{QueryBuilderFacade as QB, DB};
use serde_json::{json, Value};

// ── test-local setup helpers ──────────────────────────────────────────────────

/// Create the given table (if not already present) and DELETE all its rows so
/// every test starts from a known-empty state.
fn setup(table: &str, ddl: &str) {
    DB::statement(ddl).unwrap();
    DB::statement(&format!("DELETE FROM {table}")).unwrap();
}

/// Quick insert shorthand — panics on error.
async fn insert(table: &str, data: Value) -> u64 {
    QB::new(table).insert(data).await.unwrap()
}

// ── INSERT — id is returned, row is readable ──────────────────────────────────

#[tokio::test]
async fn test_crud_insert_returns_positive_id() {
    setup(
        "c8_insert",
        "CREATE TABLE IF NOT EXISTS c8_insert \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let id = insert("c8_insert", json!({"name": "Alice"})).await;
    assert!(id >= 1, "insert should return a positive row id, got {id}");
}

#[tokio::test]
async fn test_crud_second_insert_id_increments() {
    setup(
        "c8_insert2",
        "CREATE TABLE IF NOT EXISTS c8_insert2 \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let id1 = insert("c8_insert2", json!({"name": "Bob"})).await;
    let id2 = insert("c8_insert2", json!({"name": "Carol"})).await;
    assert!(id2 > id1, "second id ({id2}) must be greater than first ({id1})");
}

// ── FIND — present → Some(row), absent → None ───────────────────────────────

#[tokio::test]
async fn test_crud_find_returns_row_when_present() {
    setup(
        "c8_find",
        "CREATE TABLE IF NOT EXISTS c8_find \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, age INTEGER)",
    );
    let id = insert("c8_find", json!({"name": "Dave", "age": 30})).await;

    let row = QB::new("c8_find").find(id).await.unwrap();
    assert!(row.is_some(), "find({id}) should return Some");
    let row = row.unwrap();
    assert_eq!(row["name"], json!("Dave"));
    assert_eq!(row["age"], json!(30i64));
    assert_eq!(row["id"], json!(id as i64));
}

#[tokio::test]
async fn test_crud_find_returns_none_when_absent() {
    setup(
        "c8_find_none",
        "CREATE TABLE IF NOT EXISTS c8_find_none \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );

    let row = QB::new("c8_find_none").find(9999i64).await.unwrap();
    assert!(row.is_none(), "find on missing id should be None");
}

// ── UPDATE — changes only target row, returns affected count ──────────────────

#[tokio::test]
async fn test_crud_update_changes_target_row_only() {
    setup(
        "c8_update",
        "CREATE TABLE IF NOT EXISTS c8_update \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, score INTEGER)",
    );
    let id1 = insert("c8_update", json!({"name": "Eve", "score": 10})).await;
    let id2 = insert("c8_update", json!({"name": "Frank", "score": 20})).await;

    let affected = QB::new("c8_update")
        .filter("id", id1 as i64)
        .update(json!({"score": 99}))
        .await
        .unwrap();
    assert_eq!(affected, 1, "update should affect exactly 1 row");

    let eve = QB::new("c8_update").find(id1).await.unwrap().unwrap();
    let frank = QB::new("c8_update").find(id2).await.unwrap().unwrap();
    assert_eq!(eve["score"], json!(99i64), "Eve's score should be updated");
    assert_eq!(frank["score"], json!(20i64), "Frank's score must not change");
}

#[tokio::test]
async fn test_crud_update_nonexistent_returns_zero() {
    setup(
        "c8_update_miss",
        "CREATE TABLE IF NOT EXISTS c8_update_miss \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let affected = QB::new("c8_update_miss")
        .filter("id", 99999i64)
        .update(json!({"name": "Ghost"}))
        .await
        .unwrap();
    assert_eq!(affected, 0, "updating a missing row should report 0 affected");
}

// ── DELETE — removes row, returns affected count ──────────────────────────────

#[tokio::test]
async fn test_crud_delete_removes_row() {
    setup(
        "c8_delete",
        "CREATE TABLE IF NOT EXISTS c8_delete \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let id = insert("c8_delete", json!({"name": "Grace"})).await;

    let deleted = QB::new("c8_delete")
        .filter("id", id as i64)
        .delete()
        .await
        .unwrap();
    assert_eq!(deleted, 1);

    let row = QB::new("c8_delete").find(id).await.unwrap();
    assert!(row.is_none(), "row should be gone after delete");
}

#[tokio::test]
async fn test_crud_delete_nonexistent_returns_zero() {
    setup(
        "c8_delete_miss",
        "CREATE TABLE IF NOT EXISTS c8_delete_miss \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let deleted = QB::new("c8_delete_miss")
        .filter("id", 99999i64)
        .delete()
        .await
        .unwrap();
    assert_eq!(deleted, 0);
}

// ── TYPE HANDLING — null, int, float, string ──────────────────────────────────

#[tokio::test]
async fn test_crud_null_round_trip() {
    setup(
        "c8_types_null",
        "CREATE TABLE IF NOT EXISTS c8_types_null \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, notes TEXT)",
    );
    // SQLite stores JSON null as SQL NULL; should read back as JSON null.
    let id = insert("c8_types_null", json!({"notes": Value::Null})).await;
    let row = QB::new("c8_types_null").find(id).await.unwrap().unwrap();
    assert_eq!(row["notes"], Value::Null, "null should round-trip as null");
}

#[tokio::test]
async fn test_crud_integer_round_trip() {
    setup(
        "c8_types_int",
        "CREATE TABLE IF NOT EXISTS c8_types_int \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, age INTEGER)",
    );
    let id = insert("c8_types_int", json!({"age": 42i64})).await;
    let row = QB::new("c8_types_int").find(id).await.unwrap().unwrap();
    assert_eq!(row["age"], json!(42i64));
}

#[tokio::test]
async fn test_crud_float_round_trip() {
    setup(
        "c8_types_float",
        "CREATE TABLE IF NOT EXISTS c8_types_float \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, score REAL)",
    );
    let id = insert("c8_types_float", json!({"score": 3.14})).await;
    let row = QB::new("c8_types_float").find(id).await.unwrap().unwrap();
    let score = row["score"].as_f64().expect("score should be f64");
    assert!((score - 3.14).abs() < 1e-9, "float should round-trip: {score}");
}

#[tokio::test]
async fn test_crud_string_round_trip() {
    setup(
        "c8_types_str",
        "CREATE TABLE IF NOT EXISTS c8_types_str \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT)",
    );
    let id = insert("c8_types_str", json!({"label": "Hello, World!"})).await;
    let row = QB::new("c8_types_str").find(id).await.unwrap().unwrap();
    assert_eq!(row["label"], json!("Hello, World!"));
}

// ── WHERE IN executes correctly against SQLite ────────────────────────────────

#[tokio::test]
async fn test_where_in_real_sqlite_returns_correct_rows() {
    setup(
        "c8_where_in",
        "CREATE TABLE IF NOT EXISTS c8_where_in \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, status TEXT)",
    );
    for status in &["draft", "published", "archived"] {
        insert("c8_where_in", json!({"status": *status})).await;
    }

    let rows = QB::new("c8_where_in")
        .where_in("status", vec!["draft", "published"])
        .get()
        .await
        .unwrap();

    assert_eq!(rows.len(), 2, "IN clause should match exactly 2 rows");
    let statuses: Vec<&str> = rows
        .iter()
        .filter_map(|r| r["status"].as_str())
        .collect();
    assert!(statuses.contains(&"draft"), "draft should be in results");
    assert!(statuses.contains(&"published"), "published should be in results");
    assert!(!statuses.contains(&"archived"), "archived must NOT be in results");
}

#[tokio::test]
async fn test_where_not_in_real_sqlite_excludes_correct_rows() {
    setup(
        "c8_where_not_in",
        "CREATE TABLE IF NOT EXISTS c8_where_not_in \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, status TEXT)",
    );
    for status in &["draft", "published", "archived"] {
        insert("c8_where_not_in", json!({"status": *status})).await;
    }

    let rows = QB::new("c8_where_not_in")
        .where_not_in("status", vec!["archived"])
        .get()
        .await
        .unwrap();

    assert_eq!(rows.len(), 2, "NOT IN clause should exclude 1 row → 2 remain");
}

// ── WHERE NOT BETWEEN executes correctly against SQLite ───────────────────────

#[tokio::test]
async fn test_where_not_between_real_sqlite() {
    setup(
        "c8_not_between",
        "CREATE TABLE IF NOT EXISTS c8_not_between \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, score INTEGER)",
    );
    for score in [5i64, 15, 50, 90, 100] {
        insert("c8_not_between", json!({"score": score})).await;
    }

    // Scores NOT between 10 and 80 → 5, 90, 100
    let rows = QB::new("c8_not_between")
        .whereNotBetween("score", 10i64, 80i64)
        .get()
        .await
        .unwrap();

    assert_eq!(rows.len(), 3, "NOT BETWEEN 10 AND 80 should match 5, 90, 100");
    let scores: Vec<i64> = rows
        .iter()
        .filter_map(|r| r["score"].as_i64())
        .collect();
    assert!(scores.contains(&5));
    assert!(scores.contains(&90));
    assert!(scores.contains(&100));
    assert!(!scores.contains(&15));
    assert!(!scores.contains(&50));
}

/// Regression test for the precedence bug: `active = true AND NOT BETWEEN`
/// must NOT match rows where active is false but score is outside the range.
#[tokio::test]
async fn test_where_not_between_chained_with_and_has_correct_precedence() {
    setup(
        "c8_not_between_prec",
        "CREATE TABLE IF NOT EXISTS c8_not_between_prec \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, active INTEGER, score INTEGER)",
    );
    // active=1, score in range  → excluded (active OK but score in range)
    // active=1, score out of range → included
    // active=0, score in range  → excluded (active wrong)
    // active=0, score out of range → excluded (active wrong — without the fix
    //   the faulty OR would incorrectly include this row)
    for (active, score) in [(1i64, 50i64), (1, 150), (0, 50), (0, 150)] {
        insert(
            "c8_not_between_prec",
            json!({"active": active, "score": score}),
        )
        .await;
    }

    let rows = QB::new("c8_not_between_prec")
        .filter("active", 1i64)
        .whereNotBetween("score", 100i64, 200i64)
        .get()
        .await
        .unwrap();

    // Only (active=1, score=50) qualifies.
    assert_eq!(
        rows.len(),
        1,
        "only the row with active=1 AND score NOT BETWEEN 100 AND 200 should match; got {rows:?}"
    );
    assert_eq!(rows[0]["score"], json!(50i64));
}

// ── ORDER BY returns rows in the right sequence ────────────────────────────────

#[tokio::test]
async fn test_order_by_desc_returns_descending() {
    setup(
        "c8_order_desc",
        "CREATE TABLE IF NOT EXISTS c8_order_desc \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, age INTEGER)",
    );
    for age in [10i64, 30, 20] {
        insert("c8_order_desc", json!({"age": age})).await;
    }

    let rows = QB::new("c8_order_desc")
        .order_by_desc("age")
        .get()
        .await
        .unwrap();
    assert_eq!(rows[0]["age"], json!(30i64));
    assert_eq!(rows[1]["age"], json!(20i64));
    assert_eq!(rows[2]["age"], json!(10i64));
}

#[tokio::test]
async fn test_order_by_asc_returns_ascending() {
    setup(
        "c8_order_asc",
        "CREATE TABLE IF NOT EXISTS c8_order_asc \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, score INTEGER)",
    );
    for score in [30i64, 10, 20] {
        insert("c8_order_asc", json!({"score": score})).await;
    }

    let rows = QB::new("c8_order_asc")
        .order_by_asc("score")
        .get()
        .await
        .unwrap();
    assert_eq!(rows[0]["score"], json!(10i64));
    assert_eq!(rows[1]["score"], json!(20i64));
    assert_eq!(rows[2]["score"], json!(30i64));
}

// ── LIMIT + OFFSET ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_limit_offset_returns_correct_page() {
    setup(
        "c8_limit_offset",
        "CREATE TABLE IF NOT EXISTS c8_limit_offset \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, val INTEGER)",
    );
    for v in 1i64..=10 {
        insert("c8_limit_offset", json!({"val": v})).await;
    }

    let rows = QB::new("c8_limit_offset")
        .order_by_asc("val")
        .limit(3)
        .offset(3) // skip first 3 → start at val=4
        .get()
        .await
        .unwrap();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["val"], json!(4i64));
    assert_eq!(rows[1]["val"], json!(5i64));
    assert_eq!(rows[2]["val"], json!(6i64));
}

#[tokio::test]
async fn test_limit_returns_at_most_n_rows() {
    setup(
        "c8_limit",
        "CREATE TABLE IF NOT EXISTS c8_limit \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    for i in 0..20 {
        insert("c8_limit", json!({"name": format!("u{i}")})).await;
    }

    let rows = QB::new("c8_limit").limit(5).get().await.unwrap();
    assert_eq!(rows.len(), 5);
}

// ── PAGINATION METADATA ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_paginate_correct_metadata_and_data() {
    setup(
        "c8_paginate",
        "CREATE TABLE IF NOT EXISTS c8_paginate \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    for i in 1i64..=7 {
        insert("c8_paginate", json!({"name": format!("u{i}")})).await;
    }

    let page1 = QB::new("c8_paginate").paginate(3, 1).await.unwrap();
    assert_eq!(page1.total, 7, "total should be 7");
    assert_eq!(page1.per_page, 3);
    assert_eq!(page1.current_page, 1);
    assert_eq!(page1.last_page, 3, "ceil(7/3) = 3");
    assert_eq!(page1.data.len(), 3, "page 1 has 3 rows");

    let page2 = QB::new("c8_paginate").paginate(3, 2).await.unwrap();
    assert_eq!(page2.data.len(), 3, "page 2 has 3 rows");
    assert_eq!(page2.current_page, 2);

    let page3 = QB::new("c8_paginate").paginate(3, 3).await.unwrap();
    assert_eq!(page3.data.len(), 1, "page 3 has 1 remaining row");
}

#[tokio::test]
async fn test_paginate_empty_table() {
    setup(
        "c8_paginate_empty",
        "CREATE TABLE IF NOT EXISTS c8_paginate_empty \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );

    let page = QB::new("c8_paginate_empty").paginate(10, 1).await.unwrap();
    assert_eq!(page.total, 0);
    assert_eq!(page.last_page, 0, "ceil(0/10) = 0");
    assert!(page.data.is_empty());
}

#[tokio::test]
async fn test_paginate_out_of_bounds_page_returns_empty_data() {
    setup(
        "c8_paginate_oob",
        "CREATE TABLE IF NOT EXISTS c8_paginate_oob \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    for i in 0..5 {
        insert("c8_paginate_oob", json!({"name": format!("u{i}")})).await;
    }

    // Page 99 is beyond last_page (1); data must be empty, total stays correct.
    let page = QB::new("c8_paginate_oob").paginate(10, 99).await.unwrap();
    assert_eq!(page.total, 5);
    assert!(page.data.is_empty(), "beyond-last-page should return no data");
}

// ── BOUNDARY / ERROR PATHS ───────────────────────────────────────────────────

#[test]
fn test_select_from_missing_table_returns_err() {
    let result = DB::select("SELECT * FROM c8_no_such_table_xyz", &[]);
    assert!(result.is_err(), "querying a non-existent table must return Err");
}

#[tokio::test]
async fn test_first_or_fail_on_empty_table_returns_err() {
    setup(
        "c8_first_fail",
        "CREATE TABLE IF NOT EXISTS c8_first_fail \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let result = QB::new("c8_first_fail").firstOrFail().await;
    assert!(
        result.is_err(),
        "firstOrFail on empty table must return Err"
    );
    assert!(
        result.unwrap_err().contains("No record found"),
        "error message should say 'No record found'"
    );
}

#[tokio::test]
async fn test_sole_zero_rows_returns_err() {
    setup(
        "c8_sole_zero",
        "CREATE TABLE IF NOT EXISTS c8_sole_zero \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let result = QB::new("c8_sole_zero").sole().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No records found"));
}

#[tokio::test]
async fn test_sole_one_row_returns_it() {
    setup(
        "c8_sole_one",
        "CREATE TABLE IF NOT EXISTS c8_sole_one \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    insert("c8_sole_one", json!({"name": "OnlyOne"})).await;

    let result = QB::new("c8_sole_one").sole().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["name"], json!("OnlyOne"));
}

#[tokio::test]
async fn test_sole_multiple_rows_returns_err() {
    setup(
        "c8_sole_many",
        "CREATE TABLE IF NOT EXISTS c8_sole_many \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    insert("c8_sole_many", json!({"name": "First"})).await;
    insert("c8_sole_many", json!({"name": "Second"})).await;

    let result = QB::new("c8_sole_many").sole().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Multiple records found"));
}

// ── COUNT  ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_count_empty_table_is_zero() {
    setup(
        "c8_count",
        "CREATE TABLE IF NOT EXISTS c8_count \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, active INTEGER)",
    );
    assert_eq!(QB::new("c8_count").count().await.unwrap(), 0);
}

#[tokio::test]
async fn test_count_with_where_filter() {
    setup(
        "c8_count_where",
        "CREATE TABLE IF NOT EXISTS c8_count_where \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, active INTEGER)",
    );
    for active in [1i64, 1, 0, 0, 0] {
        insert("c8_count_where", json!({"active": active})).await;
    }

    let all = QB::new("c8_count_where").count().await.unwrap();
    let active_count = QB::new("c8_count_where")
        .filter("active", 1i64)
        .count()
        .await
        .unwrap();
    assert_eq!(all, 5);
    assert_eq!(active_count, 2);
}

// ── EXISTS ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_exists_false_on_empty_table() {
    setup(
        "c8_exists",
        "CREATE TABLE IF NOT EXISTS c8_exists \
         (id INTEGER PRIMARY KEY AUTOINCREMENT)",
    );
    assert!(!QB::new("c8_exists").exists().await.unwrap());
}

#[tokio::test]
async fn test_exists_true_after_insert() {
    setup(
        "c8_exists2",
        "CREATE TABLE IF NOT EXISTS c8_exists2 \
         (id INTEGER PRIMARY KEY AUTOINCREMENT)",
    );
    insert("c8_exists2", json!({})).await;
    assert!(QB::new("c8_exists2").exists().await.unwrap());
}

// ── insert_many / batch operations ───────────────────────────────────────────

#[tokio::test]
async fn test_insert_many_inserts_all_rows() {
    setup(
        "c8_many",
        "CREATE TABLE IF NOT EXISTS c8_many \
         (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)",
    );
    let inserted = QB::new("c8_many")
        .insert_many(vec![
            json!({"name": "X"}),
            json!({"name": "Y"}),
            json!({"name": "Z"}),
        ])
        .await
        .unwrap();
    assert_eq!(inserted, 3);
    assert_eq!(QB::new("c8_many").count().await.unwrap(), 3);
}
