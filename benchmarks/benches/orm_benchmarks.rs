use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use tokio::runtime::Runtime;

// # ORM Performance Benchmarks
//
// Comprehensive benchmarks for ORM operations:
// - Single record fetch
// - Bulk inserts (100, 1000, 10000 records)
// - Bulk updates
// - Relationship loading (N+1 vs eager loading)
// - Query builder performance
// - Collection operations

// Mock data structures for benchmarking
#[derive(Clone, Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
    created_at: String,
}

#[derive(Clone, Debug)]
struct Post {
    id: i64,
    user_id: i64,
    title: String,
    content: String,
}

// Benchmark: Single record fetch
fn benchmark_single_fetch(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("orm/single_fetch", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(fetch_single_user(1).await) });
    });
}

// Benchmark: Bulk inserts with varying sizes
fn benchmark_bulk_inserts(c: &mut Criterion) {
    let mut group = c.benchmark_group("orm/bulk_insert");
    let runtime = Runtime::new().unwrap();

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter_batched(
                || generate_users(size),
                |users| async move { black_box(bulk_insert_users(users).await) },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

// Benchmark: Bulk updates
fn benchmark_bulk_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("orm/bulk_update");
    let runtime = Runtime::new().unwrap();

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&runtime).iter_batched(
                || generate_user_ids(size),
                |ids| async move { black_box(bulk_update_users(ids).await) },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

// Benchmark: Relationship loading (N+1 problem)
fn benchmark_n_plus_1_queries(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("orm/n_plus_1_queries", |b| {
        b.to_async(&runtime).iter(|| async {
            // Simulate N+1 query problem
            // 1 query for users + N queries for posts (one per user)
            let users = fetch_all_users().await;

            for user in &users {
                let _posts = fetch_posts_for_user(user.id).await;
            }

            black_box(users)
        });
    });
}

// Benchmark: Eager loading (optimized)
fn benchmark_eager_loading(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("orm/eager_loading", |b| {
        b.to_async(&runtime).iter(|| async {
            // Simulate eager loading
            // 1 query for users + 1 query for all posts
            // Total: 2 queries instead of N+1
            let users = fetch_all_users_with_posts().await;

            black_box(users)
        });
    });
}

// Benchmark: Query builder operations
fn benchmark_query_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("orm/query_builder");
    let runtime = Runtime::new().unwrap();

    // Simple where clause
    group.bench_function("simple_where", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(query_users_simple().await) });
    });

    // Complex query with multiple conditions
    group.bench_function("complex_query", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(query_users_complex().await) });
    });

    // Join query
    group.bench_function("join_query", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(query_users_with_posts_join().await) });
    });

    // Aggregation query
    group.bench_function("aggregation", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(query_user_count().await) });
    });

    group.finish();
}

// Benchmark: Collection operations
fn benchmark_collection_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("orm/collection");

    // Map operation
    group.bench_function("map", |b| {
        b.iter_batched(
            || generate_users(1000),
            |users| black_box(users.into_iter().map(|u| u.email).collect::<Vec<_>>()),
            BatchSize::LargeInput,
        );
    });

    // Filter operation
    group.bench_function("filter", |b| {
        b.iter_batched(
            || generate_users(1000),
            |users| {
                black_box(
                    users
                        .into_iter()
                        .filter(|u| u.id % 2 == 0)
                        .collect::<Vec<_>>(),
                )
            },
            BatchSize::LargeInput,
        );
    });

    // Group by operation
    group.bench_function("group_by", |b| {
        b.iter_batched(
            || generate_users(1000),
            |users| black_box(group_users_by_email_domain(users)),
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

// Benchmark: Transaction operations
fn benchmark_transactions(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("orm/transaction", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(execute_transaction().await) });
    });
}

// Helper functions (mocked for benchmarking)

async fn fetch_single_user(id: i64) -> User {
    // Simulate database fetch (~1ms)
    tokio::time::sleep(tokio::time::Duration::from_micros(1000)).await;
    User {
        id,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: "2024-01-01".to_string(),
    }
}

async fn bulk_insert_users(users: Vec<User>) -> usize {
    // Simulate bulk insert (10μs per record)
    let delay = users.len() as u64 * 10;
    tokio::time::sleep(tokio::time::Duration::from_micros(delay)).await;
    users.len()
}

async fn bulk_update_users(ids: Vec<i64>) -> usize {
    // Simulate bulk update (8μs per record)
    let delay = ids.len() as u64 * 8;
    tokio::time::sleep(tokio::time::Duration::from_micros(delay)).await;
    ids.len()
}

async fn fetch_all_users() -> Vec<User> {
    // Simulate fetching 100 users
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    generate_users(100)
}

async fn fetch_posts_for_user(user_id: i64) -> Vec<Post> {
    // Simulate fetching posts (N+1 problem - called N times!)
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    vec![
        Post {
            id: 1,
            user_id,
            title: "Post 1".to_string(),
            content: "Content 1".to_string(),
        },
        Post {
            id: 2,
            user_id,
            title: "Post 2".to_string(),
            content: "Content 2".to_string(),
        },
    ]
}

async fn fetch_all_users_with_posts() -> Vec<User> {
    // Simulate eager loading (1 query for users + 1 for all posts)
    // Much faster than N+1: 5ms + 3ms = 8ms total vs 5ms + (100 * 2ms) = 205ms
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await; // users
    tokio::time::sleep(tokio::time::Duration::from_millis(3)).await; // all posts
    generate_users(100)
}

async fn query_users_simple() -> Vec<User> {
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    generate_users(10)
}

async fn query_users_complex() -> Vec<User> {
    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
    generate_users(10)
}

async fn query_users_with_posts_join() -> Vec<User> {
    tokio::time::sleep(tokio::time::Duration::from_millis(8)).await;
    generate_users(10)
}

async fn query_user_count() -> i64 {
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    1000
}

async fn execute_transaction() -> bool {
    tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
    true
}

fn generate_users(count: usize) -> Vec<User> {
    (0..count)
        .map(|i| User {
            id: i as i64,
            name: format!("User {}", i),
            email: format!("user{}@example.com", i),
            created_at: "2024-01-01".to_string(),
        })
        .collect()
}

fn generate_user_ids(count: usize) -> Vec<i64> {
    (0..count).map(|i| i as i64).collect()
}

fn group_users_by_email_domain(users: Vec<User>) -> std::collections::HashMap<String, Vec<User>> {
    let mut groups = std::collections::HashMap::new();

    for user in users {
        let domain = user
            .email
            .split('@')
            .nth(1)
            .unwrap_or("unknown")
            .to_string();
        groups.entry(domain).or_insert_with(Vec::new).push(user);
    }

    groups
}

criterion_group!(
    benches,
    benchmark_single_fetch,
    benchmark_bulk_inserts,
    benchmark_bulk_updates,
    benchmark_n_plus_1_queries,
    benchmark_eager_loading,
    benchmark_query_builder,
    benchmark_collection_operations,
    benchmark_transactions,
);

criterion_main!(benches);
