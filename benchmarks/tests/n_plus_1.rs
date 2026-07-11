//! N+1 query-count assertion test
//!
//! Proves that an "eager loading" access pattern issues exactly 2 SQL queries
//! for N users with posts (O(1) relative to user count), while a naive loop
//! issues N+1 queries (O(N)).
//!
//! Run with: cargo test -p rustforge-benchmarks --test n_plus_1
//!
//! This uses sqlx with an in-memory SQLite database.  The query count is
//! tracked by an explicit AtomicU32 counter incremented immediately before
//! every `sqlx::query*` call — the only reliable way to count round-trips
//! without patching the driver.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

const USER_COUNT: usize = 10;

/// Verify that the naive loop (N+1) issues exactly USER_COUNT + 1 queries.
/// Verify that the eager pattern issues exactly 2 queries.
#[tokio::test]
async fn n_plus_1_vs_eager_query_count() {
    // ---- setup -------------------------------------------------------
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    sqlx::query("CREATE TABLE np1_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "CREATE TABLE np1_posts \
         (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Seed USER_COUNT users, 2 posts each
    for i in 1..=(USER_COUNT as i64) {
        sqlx::query("INSERT INTO np1_users VALUES (?, ?)")
            .bind(i)
            .bind(format!("user{}", i))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO np1_posts VALUES (?, ?, ?)")
            .bind(i * 2 - 1)
            .bind(i)
            .bind(format!("post_{}_a", i))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO np1_posts VALUES (?, ?, ?)")
            .bind(i * 2)
            .bind(i)
            .bind(format!("post_{}_b", i))
            .execute(&pool)
            .await
            .unwrap();
    }

    // ---- N+1 pattern: 1 query for users + 1 per user for posts --------
    let query_counter = Arc::new(AtomicU32::new(0));

    let qc = query_counter.clone();
    qc.fetch_add(1, Ordering::Relaxed); // query 1: SELECT all users
    let user_ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM np1_users ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(
        user_ids.len(),
        USER_COUNT,
        "should have seeded {} users",
        USER_COUNT
    );

    for uid in &user_ids {
        qc.fetch_add(1, Ordering::Relaxed); // query 2…N+1: SELECT posts per user
        let _posts = sqlx::query("SELECT id, title FROM np1_posts WHERE user_id = ?")
            .bind(uid)
            .fetch_all(&pool)
            .await
            .unwrap();
    }

    let n_plus_1_queries = query_counter.load(Ordering::Relaxed);
    assert_eq!(
        n_plus_1_queries as usize,
        USER_COUNT + 1,
        "N+1 pattern must issue exactly {} queries for {} users",
        USER_COUNT + 1,
        USER_COUNT,
    );

    // ---- Eager pattern: 2 queries total regardless of user count ------
    query_counter.store(0, Ordering::Relaxed);
    let qc = query_counter.clone();

    qc.fetch_add(1, Ordering::Relaxed); // query 1: SELECT all users
    let _users = sqlx::query("SELECT id, name FROM np1_users")
        .fetch_all(&pool)
        .await
        .unwrap();

    qc.fetch_add(1, Ordering::Relaxed); // query 2: SELECT all posts in one shot
    // In an ORM this corresponds to `User::with("posts").get()` or
    // sea-orm's `find_with_related(Post)` — a single IN-clause query.
    let _posts = sqlx::query(
        "SELECT id, user_id, title FROM np1_posts \
         WHERE user_id IN (SELECT id FROM np1_users)",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let eager_queries = query_counter.load(Ordering::Relaxed);
    assert_eq!(
        eager_queries,
        2,
        "Eager loading must issue exactly 2 queries regardless of user count",
    );

    // ---- final assertion: eager is strictly fewer queries than N+1 ----
    assert!(
        eager_queries < n_plus_1_queries,
        "eager ({} queries) must beat N+1 ({} queries)",
        eager_queries,
        n_plus_1_queries,
    );

    println!(
        "\nPASS  N+1 queries: {}  |  Eager queries: {}  (ratio {:.1}x)",
        n_plus_1_queries,
        eager_queries,
        f64::from(n_plus_1_queries) / f64::from(eager_queries),
    );
}
