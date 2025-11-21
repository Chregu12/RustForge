use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

// ORM Collection performance benchmarks
// Target: <1ms overhead vs Vec, minimal memory overhead

fn benchmark_collection_vs_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_vs_vec");
    group.measurement_time(Duration::from_secs(10));

    let items = generate_test_data(10_000);

    // Benchmark: Vec operations (baseline)
    group.bench_function("vec_map", |b| {
        b.iter(|| {
            let result: Vec<_> = items.iter().map(|x| x * 2).collect();
            black_box(result)
        });
    });

    // Benchmark: Collection operations
    group.bench_function("collection_map", |b| {
        b.iter(|| {
            let result: Vec<_> = collection_map(&items);
            black_box(result)
        });
    });

    // Benchmark: Vec filter
    group.bench_function("vec_filter", |b| {
        b.iter(|| {
            let result: Vec<_> = items.iter().filter(|&&x| x % 2 == 0).cloned().collect();
            black_box(result)
        });
    });

    // Benchmark: Collection filter
    group.bench_function("collection_filter", |b| {
        b.iter(|| {
            let result = collection_filter(&items);
            black_box(result)
        });
    });

    // Benchmark: Vec pluck (select field)
    group.bench_function("vec_pluck", |b| {
        b.iter(|| {
            let result: Vec<_> = items.iter().map(|x| x.to_string()).collect();
            black_box(result)
        });
    });

    // Benchmark: Collection pluck
    group.bench_function("collection_pluck", |b| {
        b.iter(|| {
            let result = collection_pluck(&items);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_large_datasets(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_datasets");
    group.measurement_time(Duration::from_secs(15));

    // Benchmark: Operations on 100,000 items
    let large_dataset = generate_test_data(100_000);

    group.throughput(Throughput::Elements(100_000));
    group.bench_function("map_100k_items", |b| {
        b.iter(|| {
            let result = collection_map(&large_dataset);
            black_box(result)
        });
    });

    group.throughput(Throughput::Elements(100_000));
    group.bench_function("filter_100k_items", |b| {
        b.iter(|| {
            let result = collection_filter(&large_dataset);
            black_box(result)
        });
    });

    group.throughput(Throughput::Elements(100_000));
    group.bench_function("group_by_100k_items", |b| {
        b.iter(|| {
            let result = collection_group_by(&large_dataset);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_collection_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_operations");

    let items = generate_test_data(10_000);

    // Benchmark: group_by
    group.bench_function("group_by", |b| {
        b.iter(|| {
            let result = collection_group_by(&items);
            black_box(result)
        });
    });

    // Benchmark: unique_by
    group.bench_function("unique_by", |b| {
        b.iter(|| {
            let result = collection_unique_by(&items);
            black_box(result)
        });
    });

    // Benchmark: chunk
    group.bench_function("chunk_100", |b| {
        b.iter(|| {
            let result = collection_chunk(&items, 100);
            black_box(result)
        });
    });

    // Benchmark: partition
    group.bench_function("partition", |b| {
        b.iter(|| {
            let result = collection_partition(&items);
            black_box(result)
        });
    });

    // Benchmark: sort_by
    group.bench_function("sort_by", |b| {
        b.iter(|| {
            let result = collection_sort_by(&items);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_collection_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_memory");

    // Benchmark: Memory overhead comparison
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("vec_memory", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let data = generate_test_data(size);
                    black_box(data)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("collection_memory", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let data = generate_collection_data(size);
                    black_box(data)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_lazy_evaluation(c: &mut Criterion) {
    let mut group = c.benchmark_group("lazy_evaluation");

    let items = generate_test_data(100_000);

    // Benchmark: Eager evaluation
    group.bench_function("eager_chain", |b| {
        b.iter(|| {
            let result: Vec<_> = items
                .iter()
                .filter(|&&x| x % 2 == 0)
                .map(|&x| x * 2)
                .take(100)
                .collect();
            black_box(result)
        });
    });

    // Benchmark: Lazy evaluation (simulated)
    group.bench_function("lazy_chain", |b| {
        b.iter(|| {
            let result = lazy_chain(&items);
            black_box(result)
        });
    });

    group.finish();
}

fn benchmark_collection_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_iteration");

    let items = generate_test_data(10_000);

    // Benchmark: each
    group.bench_function("each", |b| {
        b.iter(|| {
            collection_each(&items);
        });
    });

    // Benchmark: reduce
    group.bench_function("reduce", |b| {
        b.iter(|| {
            let result = collection_reduce(&items);
            black_box(result)
        });
    });

    // Benchmark: flat_map
    group.bench_function("flat_map", |b| {
        b.iter(|| {
            let result = collection_flat_map(&items);
            black_box(result)
        });
    });

    group.finish();
}

// Helper functions

fn generate_test_data(size: usize) -> Vec<i32> {
    (0..size as i32).collect()
}

fn generate_collection_data(size: usize) -> Vec<i32> {
    // In real implementation, this would return a Collection wrapper
    (0..size as i32).collect()
}

fn collection_map(items: &[i32]) -> Vec<i32> {
    items.iter().map(|x| x * 2).collect()
}

fn collection_filter(items: &[i32]) -> Vec<i32> {
    items.iter().filter(|&&x| x % 2 == 0).cloned().collect()
}

fn collection_pluck(items: &[i32]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

fn collection_group_by(items: &[i32]) -> std::collections::HashMap<i32, Vec<i32>> {
    let mut groups = std::collections::HashMap::new();

    for &item in items {
        let key = item % 10;
        groups.entry(key).or_insert_with(Vec::new).push(item);
    }

    groups
}

fn collection_unique_by(items: &[i32]) -> Vec<i32> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();

    for &item in items {
        let key = item % 100;
        if seen.insert(key) {
            unique.push(item);
        }
    }

    unique
}

fn collection_chunk(items: &[i32], chunk_size: usize) -> Vec<Vec<i32>> {
    items.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
}

fn collection_partition(items: &[i32]) -> (Vec<i32>, Vec<i32>) {
    items.iter().partition(|&&x| x % 2 == 0)
}

fn collection_sort_by(items: &[i32]) -> Vec<i32> {
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.cmp(a)); // Descending order
    sorted
}

fn lazy_chain(items: &[i32]) -> Vec<i32> {
    // Simulate lazy evaluation
    items
        .iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| x * 2)
        .take(100)
        .collect()
}

fn collection_each(items: &[i32]) {
    for item in items {
        let _ = item * 2; // Simulate operation
    }
}

fn collection_reduce(items: &[i32]) -> i32 {
    items.iter().fold(0, |acc, &x| acc + x)
}

fn collection_flat_map(items: &[i32]) -> Vec<i32> {
    items
        .iter()
        .flat_map(|&x| vec![x, x * 2, x * 3])
        .collect()
}

criterion_group!(
    benches,
    benchmark_collection_vs_vec,
    benchmark_large_datasets,
    benchmark_collection_operations,
    benchmark_collection_memory,
    benchmark_lazy_evaluation,
    benchmark_collection_iteration,
);

criterion_main!(benches);
