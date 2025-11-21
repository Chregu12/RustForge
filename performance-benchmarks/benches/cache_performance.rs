use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Cache performance benchmarks
// Target: >100,000 ops/sec throughput, <1ms latency

fn benchmark_cache_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_throughput");
    group.measurement_time(Duration::from_secs(10));

    let runtime = Runtime::new().unwrap();

    // Benchmark: 100,000 SET operations
    group.throughput(Throughput::Elements(100_000));
    group.bench_function("set_100k_ops", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(perform_sets(100_000).await)
        });
    });

    // Benchmark: 100,000 GET operations
    group.throughput(Throughput::Elements(100_000));
    group.bench_function("get_100k_ops", |b| {
        b.to_async(&runtime).iter(|| async {
            // Setup: Pre-populate cache
            perform_sets(100_000).await;
            black_box(perform_gets(100_000).await)
        });
    });

    // Benchmark: Mixed operations (50% GET, 50% SET)
    group.throughput(Throughput::Elements(100_000));
    group.bench_function("mixed_100k_ops", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(perform_mixed_ops(100_000).await)
        });
    });

    group.finish();
}

fn benchmark_cache_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_latency");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Single SET latency
    group.bench_function("set_single", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(cache_set("key", "value").await)
        });
    });

    // Benchmark: Single GET latency
    group.bench_function("get_single", |b| {
        b.to_async(&runtime).iter(|| async {
            // Setup: Ensure key exists
            cache_set("key", "value").await;
            black_box(cache_get("key").await)
        });
    });

    // Benchmark: GET latency (cache miss)
    group.bench_function("get_miss", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(cache_get("nonexistent_key").await)
        });
    });

    // Benchmark: DELETE latency
    group.bench_function("delete_single", |b| {
        b.to_async(&runtime).iter(|| async {
            cache_set("key", "value").await;
            black_box(cache_delete("key").await)
        });
    });

    group.finish();
}

fn benchmark_stampede_prevention(c: &mut Criterion) {
    let mut group = c.benchmark_group("stampede_prevention");
    group.measurement_time(Duration::from_secs(15));

    let runtime = Runtime::new().unwrap();

    // Benchmark: 100 concurrent requests for same key (cache miss)
    group.bench_function("100_concurrent_same_key", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(concurrent_cache_access(100, "same_key").await)
        });
    });

    // Benchmark: Stampede prevention effectiveness
    group.bench_function("stampede_with_lock", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(stampede_with_lock(100).await)
        });
    });

    // Benchmark: Without stampede prevention (for comparison)
    group.bench_function("stampede_without_lock", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(stampede_without_lock(100).await)
        });
    });

    group.finish();
}

fn benchmark_cache_hit_rates(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_rates");

    let runtime = Runtime::new().unwrap();

    // Benchmark: 80% hit rate
    group.bench_function("hit_rate_80pct", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(simulate_hit_rate(10_000, 0.8).await)
        });
    });

    // Benchmark: 50% hit rate
    group.bench_function("hit_rate_50pct", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(simulate_hit_rate(10_000, 0.5).await)
        });
    });

    // Benchmark: 95% hit rate
    group.bench_function("hit_rate_95pct", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(simulate_hit_rate(10_000, 0.95).await)
        });
    });

    group.finish();
}

fn benchmark_distributed_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("distributed_cache");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Multiple instances accessing same cache
    group.bench_function("5_instances_concurrent", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(distributed_cache_access(5, 1_000).await)
        });
    });

    // Benchmark: Cache invalidation across instances
    group.bench_function("invalidation_across_instances", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(distributed_invalidation(5).await)
        });
    });

    group.finish();
}

fn benchmark_cache_tags(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_tags");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Tagged cache operations
    group.bench_function("set_with_tags", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(cache_set_with_tags("key", "value", &["tag1", "tag2"]).await)
        });
    });

    // Benchmark: Flush by tag
    group.bench_function("flush_by_tag", |b| {
        b.to_async(&runtime).iter(|| async {
            // Setup: Create tagged entries
            for i in 0..100 {
                cache_set_with_tags(&format!("key{}", i), "value", &["users"]).await;
            }
            black_box(flush_tag("users").await)
        });
    });

    group.finish();
}

fn benchmark_cache_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Memory usage with different entry counts
    for entry_count in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_usage", entry_count),
            entry_count,
            |b, &count| {
                b.to_async(&runtime).iter(|| async move {
                    black_box(measure_cache_memory(count).await)
                });
            },
        );
    }

    group.finish();
}

// Helper functions - Mock implementations for benchmarking

async fn perform_sets(count: usize) -> usize {
    let mut successful = 0;
    for i in 0..count {
        tokio::time::sleep(Duration::from_nanos(10)).await;
        successful += 1;
    }
    successful
}

async fn perform_gets(count: usize) -> usize {
    let mut successful = 0;
    for _ in 0..count {
        tokio::time::sleep(Duration::from_nanos(8)).await;
        successful += 1;
    }
    successful
}

async fn perform_mixed_ops(count: usize) -> usize {
    let mut successful = 0;
    for i in 0..count {
        if i % 2 == 0 {
            tokio::time::sleep(Duration::from_nanos(10)).await; // SET
        } else {
            tokio::time::sleep(Duration::from_nanos(8)).await; // GET
        }
        successful += 1;
    }
    successful
}

async fn cache_set(_key: &str, _value: &str) -> bool {
    tokio::time::sleep(Duration::from_micros(1)).await;
    true
}

async fn cache_get(_key: &str) -> Option<String> {
    tokio::time::sleep(Duration::from_nanos(500)).await;
    Some("value".to_string())
}

async fn cache_delete(_key: &str) -> bool {
    tokio::time::sleep(Duration::from_nanos(800)).await;
    true
}

async fn concurrent_cache_access(concurrent_requests: usize, key: &str) -> usize {
    let mut handles = vec![];

    for _ in 0..concurrent_requests {
        let key = key.to_string();
        let handle = tokio::spawn(async move {
            cache_get(&key).await.is_some()
        });
        handles.push(handle);
    }

    let mut successful = 0;
    for handle in handles {
        if handle.await.unwrap() {
            successful += 1;
        }
    }
    successful
}

async fn stampede_with_lock(concurrent_requests: usize) -> usize {
    // Simulate stampede prevention with lock
    // Only one computation should happen
    let computation_count = Arc::new(tokio::sync::Mutex::new(0));
    let cache_value = Arc::new(tokio::sync::Mutex::new(None::<String>));

    let mut handles = vec![];

    for _ in 0..concurrent_requests {
        let computation_count = computation_count.clone();
        let cache_value = cache_value.clone();

        let handle = tokio::spawn(async move {
            let mut cache = cache_value.lock().await;

            if cache.is_none() {
                // Simulate expensive computation
                tokio::time::sleep(Duration::from_millis(10)).await;
                *computation_count.lock().await += 1;
                *cache = Some("computed_value".to_string());
            }

            cache.clone().unwrap()
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    *computation_count.lock().await
}

async fn stampede_without_lock(concurrent_requests: usize) -> usize {
    // Simulate stampede without lock
    // Multiple computations will happen
    let computation_count = Arc::new(tokio::sync::Mutex::new(0));

    let mut handles = vec![];

    for _ in 0..concurrent_requests {
        let computation_count = computation_count.clone();

        let handle = tokio::spawn(async move {
            // Simulate expensive computation (no locking)
            tokio::time::sleep(Duration::from_millis(10)).await;
            *computation_count.lock().await += 1;
            "computed_value".to_string()
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    *computation_count.lock().await
}

async fn simulate_hit_rate(requests: usize, hit_rate: f64) -> usize {
    let mut hits = 0;

    for i in 0..requests {
        let is_hit = (i as f64 / requests as f64) < hit_rate;

        if is_hit {
            tokio::time::sleep(Duration::from_nanos(500)).await; // Fast (cache hit)
            hits += 1;
        } else {
            tokio::time::sleep(Duration::from_micros(10)).await; // Slow (cache miss)
        }
    }

    hits
}

async fn distributed_cache_access(instances: usize, ops_per_instance: usize) -> usize {
    let mut handles = vec![];

    for _ in 0..instances {
        let handle = tokio::spawn(async move {
            let mut successful = 0;
            for _ in 0..ops_per_instance {
                tokio::time::sleep(Duration::from_nanos(500)).await;
                successful += 1;
            }
            successful
        });
        handles.push(handle);
    }

    let mut total = 0;
    for handle in handles {
        total += handle.await.unwrap();
    }
    total
}

async fn distributed_invalidation(instances: usize) -> bool {
    let mut handles = vec![];

    for _ in 0..instances {
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_micros(50)).await;
            true
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    true
}

async fn cache_set_with_tags(_key: &str, _value: &str, _tags: &[&str]) -> bool {
    tokio::time::sleep(Duration::from_micros(2)).await;
    true
}

async fn flush_tag(_tag: &str) -> bool {
    tokio::time::sleep(Duration::from_micros(50)).await;
    true
}

async fn measure_cache_memory(entries: usize) -> usize {
    tokio::time::sleep(Duration::from_micros(entries as u64 / 100)).await;
    entries * 128 // Simulate ~128 bytes per entry
}

criterion_group!(
    benches,
    benchmark_cache_throughput,
    benchmark_cache_latency,
    benchmark_stampede_prevention,
    benchmark_cache_hit_rates,
    benchmark_distributed_cache,
    benchmark_cache_tags,
    benchmark_cache_memory,
);

criterion_main!(benches);
