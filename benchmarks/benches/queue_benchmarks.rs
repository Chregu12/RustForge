use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

// # Queue Performance Benchmarks
//
// Benchmarks for job queue operations:
// - Job dispatch rate
// - Job processing throughput
// - Queue operations (push/pop)
// - Batch job performance

struct MockJob {
    id: u64,
    payload: Vec<u8>,
}

struct MockQueue {
    counter: Arc<AtomicU64>,
}

impl MockQueue {
    fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn push(&self, _job: MockJob) {
        // Simulate queue push (Redis: ~200μs)
        tokio::time::sleep(tokio::time::Duration::from_micros(200)).await;
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    async fn pop(&self) -> Option<MockJob> {
        // Simulate queue pop
        tokio::time::sleep(tokio::time::Duration::from_micros(200)).await;
        if self.counter.load(Ordering::Relaxed) > 0 {
            self.counter.fetch_sub(1, Ordering::Relaxed);
            Some(MockJob {
                id: 1,
                payload: vec![0; 100],
            })
        } else {
            None
        }
    }

    async fn process_job(&self, _job: MockJob) {
        // Simulate job processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

fn benchmark_job_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/dispatch");
    let runtime = Runtime::new().unwrap();

    for count in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let queue = MockQueue::new();

            b.to_async(&runtime).iter(|| async {
                for i in 0..count {
                    queue
                        .push(MockJob {
                            id: i,
                            payload: vec![0; 100],
                        })
                        .await;
                }
            });
        });
    }

    group.finish();
}

fn benchmark_job_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/process");
    let runtime = Runtime::new().unwrap();

    for count in [10, 50, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let queue = Arc::new(MockQueue::new());

            b.to_async(&runtime).iter(|| {
                let queue = queue.clone();
                async move {
                    // Pre-populate queue
                    for i in 0..count {
                        queue
                            .push(MockJob {
                                id: i,
                                payload: vec![0; 100],
                            })
                            .await;
                    }

                    // Process jobs
                    for _ in 0..count {
                        if let Some(job) = queue.pop().await {
                            queue.process_job(job).await;
                        }
                    }
                }
            });
        });
    }

    group.finish();
}

fn benchmark_concurrent_workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue/workers");
    let runtime = Runtime::new().unwrap();

    for workers in [1, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(workers),
            workers,
            |b, &workers| {
                let queue = Arc::new(MockQueue::new());

                b.to_async(&runtime).iter(|| {
                    let queue = queue.clone();
                    async move {
                        // Pre-populate with 100 jobs
                        for i in 0..100 {
                            queue
                                .push(MockJob {
                                    id: i,
                                    payload: vec![0; 100],
                                })
                                .await;
                        }

                        // Spawn workers
                        let mut handles = Vec::new();
                        for _ in 0..workers {
                            let q = queue.clone();
                            handles.push(tokio::spawn(async move {
                                for _ in 0..100 / workers {
                                    if let Some(job) = q.pop().await {
                                        q.process_job(job).await;
                                    }
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

criterion_group!(
    benches,
    benchmark_job_dispatch,
    benchmark_job_processing,
    benchmark_concurrent_workers
);
criterion_main!(benches);
