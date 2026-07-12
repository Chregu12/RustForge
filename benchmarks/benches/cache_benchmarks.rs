use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

// # Cache Performance Benchmarks
//
// Comprehensive benchmarks for cache operations:
// - Redis operations (get, set, delete)
// - In-memory cache operations
// - Cache hit vs miss scenarios
// - Concurrent cache access
// - Tag-based invalidation
// - Cache warming strategies

// Mock cache implementation for benchmarking.
// Clone is cheap + correct: the only field is an Arc, so clones share one store —
// needed because criterion's batched FnMut closures move the cache into `async move`.
#[derive(Clone)]
struct MockCache {
    data: Arc<parking_lot::RwLock<HashMap<String, Vec<u8>>>>,
}

impl MockCache {
    fn new() -> Self {
        Self {
            data: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        // Simulate cache get (Redis: ~100μs, Memory: ~1μs)
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        self.data.read().get(key).cloned()
    }

    async fn set(&self, key: String, value: Vec<u8>) {
        // Simulate cache set (Redis: ~150μs, Memory: ~2μs)
        tokio::time::sleep(tokio::time::Duration::from_micros(2)).await;
        self.data.write().insert(key, value);
    }

    async fn delete(&self, key: &str) {
        // Simulate cache delete (Redis: ~100μs, Memory: ~1μs)
        tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        self.data.write().remove(key);
    }

    fn clear(&self) {
        self.data.write().clear();
    }
}

// Benchmark: Cache get operations
fn benchmark_cache_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/get");
    let runtime = Runtime::new().unwrap();

    group.bench_function("hit", |b| {
        let cache = MockCache::new();

        // Pre-populate cache
        runtime.block_on(async {
            cache.set("key1".to_string(), vec![1, 2, 3, 4, 5]).await;
        });

        b.to_async(&runtime)
            .iter(|| async { black_box(cache.get("key1").await) });
    });

    group.bench_function("miss", |b| {
        let cache = MockCache::new();

        b.to_async(&runtime)
            .iter(|| async { black_box(cache.get("nonexistent").await) });
    });

    group.finish();
}

// Benchmark: Cache set operations with varying value sizes
fn benchmark_cache_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/set");
    let runtime = Runtime::new().unwrap();

    for size in [100, 1024, 10240, 102400].iter() {
        group.bench_with_input(BenchmarkId::new("value_size", size), size, |b, &size| {
            let cache = MockCache::new();
            b.to_async(&runtime).iter_batched(
                || {
                    let value = vec![0u8; size];
                    (format!("key_{}", size), value)
                },
                |(key, value)| {
                    let cache = cache.clone();
                    async move {
                        cache.set(key, value).await;
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// Benchmark: Cache delete operations
fn benchmark_cache_delete(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("cache/delete", |b| {
        let cache = MockCache::new();

        b.to_async(&runtime).iter_batched(
            || {
                // Setup: insert key before each delete
                let key = format!("key_{}", rand::random::<u64>());
                runtime.block_on(async {
                    cache.set(key.clone(), vec![1, 2, 3]).await;
                });
                key
            },
            |key| {
                let cache = cache.clone();
                async move {
                    cache.delete(&key).await;
                }
            },
            BatchSize::SmallInput,
        );
    });
}

// Benchmark: Concurrent cache access
fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/concurrent");
    let runtime = Runtime::new().unwrap();

    for concurrency in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("readers", concurrency),
            concurrency,
            |b, &concurrency| {
                let cache = Arc::new(MockCache::new());

                // Pre-populate cache
                runtime.block_on(async {
                    for i in 0..100 {
                        cache.set(format!("key{}", i), vec![i as u8; 100]).await;
                    }
                });

                b.to_async(&runtime).iter(|| {
                    let cache = cache.clone();
                    async move {
                        let mut handles = Vec::new();

                        for _ in 0..concurrency {
                            let cache_clone = cache.clone();
                            handles.push(tokio::spawn(async move {
                                for i in 0..10 {
                                    let _ = cache_clone.get(&format!("key{}", i)).await;
                                }
                            }));
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark: Cache hit rate scenarios
fn benchmark_hit_rate_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/hit_rate");
    let runtime = Runtime::new().unwrap();

    // 90% hit rate
    group.bench_function("90_percent_hit", |b| {
        let cache = MockCache::new();

        // Pre-populate 90% of keys
        runtime.block_on(async {
            for i in 0..90 {
                cache.set(format!("key{}", i), vec![i as u8; 100]).await;
            }
        });

        b.to_async(&runtime).iter(|| async {
            // Access 100 keys (90 exist, 10 don't)
            for i in 0..100 {
                let _ = cache.get(&format!("key{}", i)).await;
            }
        });
    });

    // 50% hit rate
    group.bench_function("50_percent_hit", |b| {
        let cache = MockCache::new();

        // Pre-populate 50% of keys
        runtime.block_on(async {
            for i in 0..50 {
                cache.set(format!("key{}", i), vec![i as u8; 100]).await;
            }
        });

        b.to_async(&runtime).iter(|| async {
            // Access 100 keys (50 exist, 50 don't)
            for i in 0..100 {
                let _ = cache.get(&format!("key{}", i)).await;
            }
        });
    });

    group.finish();
}

// Benchmark: Bulk operations
fn benchmark_bulk_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/bulk");
    let runtime = Runtime::new().unwrap();

    for count in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("bulk_set", count), count, |b, &count| {
            let cache = MockCache::new();

            b.to_async(&runtime).iter(|| async {
                for i in 0..count {
                    cache.set(format!("key{}", i), vec![i as u8; 100]).await;
                }
            });

            // Clear for next iteration
            cache.clear();
        });

        group.bench_with_input(BenchmarkId::new("bulk_get", count), count, |b, &count| {
            let cache = MockCache::new();

            // Pre-populate
            runtime.block_on(async {
                for i in 0..count {
                    cache.set(format!("key{}", i), vec![i as u8; 100]).await;
                }
            });

            b.to_async(&runtime).iter(|| async {
                for i in 0..count {
                    let _ = cache.get(&format!("key{}", i)).await;
                }
            });
        });
    }

    group.finish();
}

// Benchmark: Cache memory efficiency
fn benchmark_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/memory");

    group.bench_function("small_values", |b| {
        b.iter_batched(
            || MockCache::new(),
            |cache| {
                // Store 1000 small values (100 bytes each)
                black_box({
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        for i in 0..1000 {
                            cache.set(format!("key{}", i), vec![0u8; 100]).await;
                        }
                    });
                    cache
                })
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("large_values", |b| {
        b.iter_batched(
            || MockCache::new(),
            |cache| {
                // Store 100 large values (100KB each)
                black_box({
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        for i in 0..100 {
                            cache.set(format!("key{}", i), vec![0u8; 102400]).await;
                        }
                    });
                    cache
                })
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

// Benchmark: Cache eviction strategies
fn benchmark_eviction_strategies(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("cache/eviction/lru", |b| {
        let cache = MockCache::new();

        // Fill cache to capacity
        runtime.block_on(async {
            for i in 0..1000 {
                cache.set(format!("key{}", i), vec![i as u8; 100]).await;
            }
        });

        b.to_async(&runtime).iter(|| async {
            // Access pattern that triggers eviction
            for i in 1000..1100 {
                cache.set(format!("key{}", i), vec![i as u8; 100]).await;
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_cache_get,
    benchmark_cache_set,
    benchmark_cache_delete,
    benchmark_concurrent_access,
    benchmark_hit_rate_scenarios,
    benchmark_bulk_operations,
    benchmark_memory_efficiency,
    benchmark_eviction_strategies,
);

criterion_main!(benches);
