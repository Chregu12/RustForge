use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Queue performance benchmarks
// Target: >10,000 jobs/sec throughput, ~1ms latency

fn benchmark_queue_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_throughput");
    group.measurement_time(Duration::from_secs(10));

    let runtime = Runtime::new().unwrap();

    // Benchmark: Push 10,000 jobs
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("push_10k_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(push_jobs(10_000).await)
        });
    });

    // Benchmark: Pop 10,000 jobs
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("pop_10k_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            // Setup: Push jobs first
            push_jobs(10_000).await;
            black_box(pop_jobs(10_000).await)
        });
    });

    group.finish();
}

fn benchmark_queue_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_latency");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Single job push latency
    group.bench_function("push_single_job", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(push_single_job().await)
        });
    });

    // Benchmark: Single job pop latency
    group.bench_function("pop_single_job", |b| {
        b.to_async(&runtime).iter(|| async {
            // Setup: Ensure job is available
            push_single_job().await;
            black_box(pop_single_job().await)
        });
    });

    // Benchmark: Round-trip latency (push + pop)
    group.bench_function("roundtrip_latency", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(roundtrip_job().await)
        });
    });

    group.finish();
}

fn benchmark_concurrent_workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_workers");
    group.measurement_time(Duration::from_secs(15));

    let runtime = Runtime::new().unwrap();

    // Benchmark: 10 workers × 1000 jobs each
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("10_workers_1000_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(concurrent_workers(10, 1_000).await)
        });
    });

    // Benchmark: 5 workers × 2000 jobs each
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("5_workers_2000_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(concurrent_workers(5, 2_000).await)
        });
    });

    // Benchmark: 20 workers × 500 jobs each
    group.throughput(Throughput::Elements(10_000));
    group.bench_function("20_workers_500_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(concurrent_workers(20, 500).await)
        });
    });

    group.finish();
}

fn benchmark_delayed_jobs(c: &mut Criterion) {
    let mut group = c.benchmark_group("delayed_jobs");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Schedule delayed job
    group.bench_function("schedule_delayed_job", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(schedule_delayed_job(Duration::from_secs(60)).await)
        });
    });

    // Benchmark: Process delayed jobs
    group.bench_function("process_delayed_jobs", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(process_delayed_jobs().await)
        });
    });

    group.finish();
}

fn benchmark_job_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("job_persistence");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Job survives restart
    group.bench_function("persist_and_recover", |b| {
        b.to_async(&runtime).iter(|| async {
            black_box(persist_and_recover().await)
        });
    });

    group.finish();
}

fn benchmark_queue_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_memory");

    let runtime = Runtime::new().unwrap();

    // Benchmark: Memory usage per 1000 jobs
    for job_count in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("memory_per_jobs", job_count),
            job_count,
            |b, &count| {
                b.to_async(&runtime).iter(|| async move {
                    black_box(measure_memory_usage(count).await)
                });
            },
        );
    }

    group.finish();
}

// Helper functions - Mock implementations for benchmarking

async fn push_jobs(count: usize) -> usize {
    // Simulate pushing jobs to queue
    // In real implementation, this would use RedisQueue or MemoryQueue
    let mut successful = 0;
    for _ in 0..count {
        tokio::time::sleep(Duration::from_micros(1)).await;
        successful += 1;
    }
    successful
}

async fn pop_jobs(count: usize) -> usize {
    // Simulate popping jobs from queue
    let mut successful = 0;
    for _ in 0..count {
        tokio::time::sleep(Duration::from_micros(1)).await;
        successful += 1;
    }
    successful
}

async fn push_single_job() -> bool {
    tokio::time::sleep(Duration::from_micros(5)).await;
    true
}

async fn pop_single_job() -> bool {
    tokio::time::sleep(Duration::from_micros(5)).await;
    true
}

async fn roundtrip_job() -> bool {
    push_single_job().await;
    pop_single_job().await
}

async fn concurrent_workers(workers: usize, jobs_per_worker: usize) -> usize {
    let mut handles = vec![];

    for _ in 0..workers {
        let handle = tokio::spawn(async move {
            let mut processed = 0;
            for _ in 0..jobs_per_worker {
                tokio::time::sleep(Duration::from_micros(1)).await;
                processed += 1;
            }
            processed
        });
        handles.push(handle);
    }

    let mut total = 0;
    for handle in handles {
        total += handle.await.unwrap();
    }
    total
}

async fn schedule_delayed_job(_delay: Duration) -> bool {
    tokio::time::sleep(Duration::from_micros(10)).await;
    true
}

async fn process_delayed_jobs() -> usize {
    // Simulate processing 100 delayed jobs
    tokio::time::sleep(Duration::from_micros(500)).await;
    100
}

async fn persist_and_recover() -> bool {
    // Simulate job persistence
    tokio::time::sleep(Duration::from_micros(50)).await;
    true
}

async fn measure_memory_usage(job_count: usize) -> usize {
    // Simulate memory measurement
    tokio::time::sleep(Duration::from_micros(job_count as u64)).await;
    job_count
}

criterion_group!(
    benches,
    benchmark_queue_throughput,
    benchmark_queue_latency,
    benchmark_concurrent_workers,
    benchmark_delayed_jobs,
    benchmark_job_persistence,
    benchmark_queue_memory,
);

criterion_main!(benches);
