# Performance Benchmarks

Comprehensive performance benchmarking suite for Rust DX-Framework critical components.

## Overview

This benchmark suite tests the performance of:

- **Queue Backend** (Redis & Memory)
- **Cache Backend** (Redis & Memory)
- **ORM Collections**
- **Real-world Scenarios**

## Quick Start

### Run All Benchmarks

```bash
cd performance-benchmarks
cargo bench
```

### Run Specific Benchmark

```bash
# Queue performance
cargo bench --bench queue_performance

# Cache performance
cargo bench --bench cache_performance

# ORM collections
cargo bench --bench orm_collection_performance

# Real-world scenarios
cargo bench --bench realworld_scenarios
```

## Benchmark Suites

### 1. Queue Performance (`queue_performance`)

Tests Redis queue backend performance:

- **Throughput:** Push/pop 10,000 jobs
- **Latency:** Single job operations (p50, p95, p99)
- **Concurrent Workers:** 10 workers × 1,000 jobs
- **Delayed Jobs:** Scheduling and processing
- **Persistence:** Job survival across restarts
- **Memory:** Usage per 1,000 jobs

**Target:** >10,000 jobs/sec, ~1ms latency

### 2. Cache Performance (`cache_performance`)

Tests Redis cache backend performance:

- **Throughput:** 100,000 SET/GET operations
- **Latency:** Single operation latency (p50, p95, p99)
- **Stampede Prevention:** 100 concurrent requests for same key
- **Hit Rates:** 50%, 80%, 95% hit rate scenarios
- **Distributed Cache:** 5 instances concurrent access
- **Tags:** Tagged cache operations
- **Memory:** Usage per entry

**Target:** >100,000 ops/sec, <1ms latency

### 3. ORM Collection Performance (`orm_collection_performance`)

Tests ORM collection overhead and performance:

- **vs Vec:** Compare map, filter, pluck operations
- **Large Datasets:** 100,000 items operations
- **Collection Operations:** group_by, unique_by, chunk, partition
- **Memory Overhead:** Comparison with Vec
- **Lazy Evaluation:** Performance gains

**Target:** <1ms overhead vs Vec, minimal memory overhead

### 4. Real-World Scenarios (`realworld_scenarios`)

Tests realistic production workloads:

- **E-commerce Checkout:** 1,000 concurrent users
- **Email Queue:** 10,000 emails/minute
- **API Cache:** 10,000 req/sec with 80% hit rate
- **User Sessions:** 1,000 concurrent sessions
- **Database Operations:** CRUD operations
- **Background Jobs:** Image processing, reports
- **API Gateway:** Request routing, rate limiting

## Performance Targets

| Component | Metric | Target | Achieved |
|-----------|--------|--------|----------|
| Queue | Throughput | >10,000 jobs/sec | 15,234 jobs/sec ✅ |
| Queue | Latency (p50) | ~1ms | 0.65ms ✅ |
| Cache | Throughput | >100,000 ops/sec | 178,571 ops/sec ✅ |
| Cache | Latency (p50) | <1ms | 0.28ms ✅ |
| Collections | Overhead | <1ms | 0.046ms ✅ |

## Running with Custom Parameters

### Quick Run (Fast Testing)

```bash
cargo bench -- --sample-size 10 --quick
```

### Detailed Run (Accurate Results)

```bash
cargo bench -- --sample-size 100
```

### Save Baseline

```bash
cargo bench -- --save-baseline main
```

### Compare with Baseline

```bash
cargo bench -- --baseline main
```

## Interpreting Results

### Throughput

- **Good:** >80% of target
- **Excellent:** >100% of target
- **Outstanding:** >150% of target

### Latency

- **p50:** Median latency (50% of requests)
- **p95:** 95th percentile (95% of requests faster than this)
- **p99:** 99th percentile (99% of requests faster than this)

**Good:** <2x target
**Excellent:** <target
**Outstanding:** <0.5x target

### Memory

- **Good:** <20% overhead vs baseline
- **Excellent:** <10% overhead
- **Outstanding:** <5% overhead

## CI/CD Integration

### GitHub Actions

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: |
          cd performance-benchmarks
          cargo bench -- --save-baseline pr-${{ github.event.pull_request.number }}
```

## Performance Monitoring

### Recommended Tools

- **Metrics:** Prometheus + Grafana
- **Tracing:** Jaeger / OpenTelemetry
- **Profiling:** `cargo flamegraph`, `perf`
- **Memory:** `valgrind`, `heaptrack`

### Production Monitoring

Set up alerts for:

- Queue throughput < 8,000 jobs/sec
- Queue latency (p95) > 5ms
- Cache throughput < 80,000 ops/sec
- Cache hit rate < 70%
- API response time (p95) > 200ms

## Benchmark Reports

Full performance reports are available in:

- **Full Report:** [`/docs/PERFORMANCE_BENCHMARK_REPORT.md`](../docs/PERFORMANCE_BENCHMARK_REPORT.md)
- **Summary:** [`/docs/PERFORMANCE_SUMMARY.md`](../docs/PERFORMANCE_SUMMARY.md)

## Optimization Guidelines

### When to Optimize

1. **Throughput** < 80% of target
2. **Latency (p95)** > 2x target
3. **Memory usage** > 20% overhead
4. **Error rate** > 1%

### What to Optimize First

1. **Hot paths:** Functions called most frequently
2. **Bottlenecks:** Operations with highest latency
3. **Memory leaks:** Unbounded growth
4. **Lock contention:** High concurrency conflicts

### How to Optimize

1. **Profile first:** Use `cargo flamegraph` or `perf`
2. **Measure impact:** Run benchmarks before/after
3. **Document changes:** Update performance reports
4. **Regression test:** Add benchmark for the issue

## Contributing

When adding new benchmarks:

1. Add benchmark file to `/benches/`
2. Update `Cargo.toml` with benchmark definition
3. Document expected performance targets
4. Run benchmark and verify results
5. Update performance reports

## License

MIT OR Apache-2.0
