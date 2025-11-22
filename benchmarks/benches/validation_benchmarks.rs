use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use tokio::runtime::Runtime;

// # Validation Performance Benchmarks
//
// Benchmarks for validation rules:
// - Simple validation rules (required, email, numeric)
// - Complex validation (regex, custom rules)
// - Database validation rules (unique, exists)
// - Validation of large datasets

async fn validate_required(value: &str) -> bool {
    !value.is_empty()
}

async fn validate_email(value: &str) -> bool {
    tokio::time::sleep(tokio::time::Duration::from_micros(5)).await;
    value.contains('@') && value.contains('.')
}

async fn validate_regex(value: &str, _pattern: &str) -> bool {
    tokio::time::sleep(tokio::time::Duration::from_micros(20)).await;
    true
}

async fn validate_unique_db(value: &str) -> bool {
    // Simulate database query
    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
    value != "taken@example.com"
}

fn benchmark_simple_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation/simple");
    let runtime = Runtime::new().unwrap();

    group.bench_function("required", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(validate_required("test value").await) });
    });

    group.bench_function("email", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(validate_email("user@example.com").await) });
    });

    group.finish();
}

fn benchmark_complex_rules(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation/complex");
    let runtime = Runtime::new().unwrap();

    group.bench_function("regex", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(validate_regex("test123", r"^\w+\d+$").await) });
    });

    group.bench_function("unique_database", |b| {
        b.to_async(&runtime)
            .iter(|| async { black_box(validate_unique_db("new@example.com").await) });
    });

    group.finish();
}

fn benchmark_bulk_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation/bulk");
    let runtime = Runtime::new().unwrap();

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.to_async(&runtime).iter(|| async move {
                for i in 0..count {
                    let _ = validate_email(&format!("user{}@example.com", i)).await;
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_simple_rules,
    benchmark_complex_rules,
    benchmark_bulk_validation
);
criterion_main!(benches);
