/// Comprehensive Performance Benchmarks for RustForge Framework
///
/// This benchmark suite measures:
/// - Command dispatch overhead
/// - Service container resolution
/// - Input parsing performance
/// - Cache operations
/// - Database connection pooling
///
/// Run with: cargo bench --bench command_dispatch

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;

// Mock implementations for benchmarking
struct MockApp;

impl MockApp {
    fn new() -> Self {
        Self
    }

    async fn dispatch(&self, _command: &str, _args: Vec<String>) -> Result<(), String> {
        // Simulate command execution
        tokio::time::sleep(Duration::from_micros(10)).await;
        Ok(())
    }
}

/// Benchmark command dispatch with varying batch sizes
fn benchmark_command_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("command_dispatch");
    group.measurement_time(Duration::from_secs(10));

    for size in &[1, 10, 100, 1000] {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let app = MockApp::new();
                for i in 0..size {
                    app.dispatch(
                        black_box("test:command"),
                        black_box(vec![format!("arg{}", i)]),
                    )
                    .await
                    .unwrap();
                }
            });
        });
    }
    group.finish();
}

/// Benchmark service container resolution
fn benchmark_service_resolution(c: &mut Criterion) {
    use foundry_service_container::{Container, FastContainer};

    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("service_resolution");

    #[derive(Clone)]
    struct TestService;

    // Standard HashMap-based container
    group.bench_function("standard_container", |b| {
        b.to_async(&rt).iter(|| async {
            let container = Container::new();
            container
                .singleton("test", || Ok(TestService))
                .await
                .unwrap();

            for _ in 0..100 {
                let _service: std::sync::Arc<TestService> =
                    container.resolve(black_box("test")).await.unwrap();
            }
        });
    });

    // FxHashMap-based fast container
    group.bench_function("fast_container", |b| {
        b.to_async(&rt).iter(|| async {
            let container = FastContainer::new();
            container
                .singleton("test", || Ok(TestService))
                .await
                .unwrap();

            for _ in 0..100 {
                let _service: std::sync::Arc<TestService> =
                    container.resolve(black_box("test")).await.unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark input parsing (standard vs optimized)
fn benchmark_input_parsing(c: &mut Criterion) {
    use foundry_api::input::InputParser;
    use foundry_api::optimized_input::OptimizedInputParser;

    let mut group = c.benchmark_group("input_parsing");

    let small_args = vec![
        "migrate:run".to_string(),
        "--force".to_string(),
        "--verbose".to_string(),
    ];

    let large_args = vec![
        "command".to_string(),
        "--opt1=val1".to_string(),
        "--opt2=val2".to_string(),
        "--opt3=val3".to_string(),
        "--opt4=val4".to_string(),
        "--opt5=val5".to_string(),
        "--flag1".to_string(),
        "--flag2".to_string(),
        "--flag3".to_string(),
        "arg1".to_string(),
        "arg2".to_string(),
    ];

    // Standard Vec-based parser
    group.bench_function("standard_small", |b| {
        b.iter(|| {
            let parser = InputParser::from_args(black_box(&small_args));
            black_box(parser.first_argument());
            black_box(parser.has_flag("force"));
        });
    });

    group.bench_function("standard_large", |b| {
        b.iter(|| {
            let parser = InputParser::from_args(black_box(&large_args));
            black_box(parser.first_argument());
            black_box(parser.option("opt3"));
        });
    });

    // SmallVec-based optimized parser
    group.bench_function("optimized_small", |b| {
        b.iter(|| {
            let parser = OptimizedInputParser::from_args(black_box(&small_args));
            black_box(parser.first_argument());
            black_box(parser.has_flag("force"));
        });
    });

    group.bench_function("optimized_large", |b| {
        b.iter(|| {
            let parser = OptimizedInputParser::from_args(black_box(&large_args));
            black_box(parser.first_argument());
            black_box(parser.option("opt3"));
        });
    });

    group.finish();
}

/// Benchmark zero-copy deserialization
fn benchmark_zero_copy_cache(c: &mut Criterion) {
    use foundry_cache::zero_copy::{ZeroCopyCache, CachedData};

    let mut group = c.benchmark_group("cache_serialization");

    let cache = ZeroCopyCache::new();

    let small_data = CachedData::new(
        "small_key".to_string(),
        vec![1, 2, 3, 4, 5],
    );

    let large_data = CachedData::new(
        "large_key".to_string(),
        vec![42u8; 10_000],
    );

    // Serialize small data
    group.bench_function("serialize_small", |b| {
        b.iter(|| {
            cache.serialize(black_box(&small_data)).unwrap()
        });
    });

    // Serialize large data
    group.bench_function("serialize_large", |b| {
        b.iter(|| {
            cache.serialize(black_box(&large_data)).unwrap()
        });
    });

    // Zero-copy deserialize
    let small_bytes = cache.serialize(&small_data).unwrap();
    group.bench_function("zero_copy_deserialize_small", |b| {
        b.iter(|| {
            cache.deserialize_zero_copy::<CachedData>(black_box(&small_bytes)).unwrap()
        });
    });

    let large_bytes = cache.serialize(&large_data).unwrap();
    group.bench_function("zero_copy_deserialize_large", |b| {
        b.iter(|| {
            cache.deserialize_zero_copy::<CachedData>(black_box(&large_bytes)).unwrap()
        });
    });

    group.finish();
}

/// Benchmark string identifier overhead (String vs Cow)
fn benchmark_string_identifiers(c: &mut Criterion) {
    use foundry_domain::cow_identifiers::CommandId;

    let mut group = c.benchmark_group("string_identifiers");

    // String-based (always allocates)
    group.bench_function("string_clone", |b| {
        b.iter(|| {
            let s = "migrate:run".to_string();
            for _ in 0..100 {
                let _cloned = black_box(s.clone());
            }
        });
    });

    // Cow-based (zero allocation for borrowed)
    group.bench_function("cow_borrowed", |b| {
        b.iter(|| {
            let id = CommandId::borrowed("migrate:run");
            for _ in 0..100 {
                let _cloned = black_box(id.clone());
            }
        });
    });

    // Cow-based owned
    group.bench_function("cow_owned", |b| {
        b.iter(|| {
            let id = CommandId::owned("migrate:run".to_string());
            for _ in 0..100 {
                let _cloned = black_box(id.clone());
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_command_dispatch,
    benchmark_service_resolution,
    benchmark_input_parsing,
    benchmark_zero_copy_cache,
    benchmark_string_identifiers,
);
criterion_main!(benches);
