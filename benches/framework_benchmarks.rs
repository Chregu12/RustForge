use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::time::Duration;

// Command execution benchmarks
fn benchmark_command_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_execution");

    group.bench_function("simple_command", |b| {
        b.iter(|| {
            // Simulate simple command execution
            black_box(execute_simple_command())
        });
    });

    group.bench_function("complex_command", |b| {
        b.iter(|| {
            // Simulate complex command execution
            black_box(execute_complex_command())
        });
    });

    group.finish();
}

// Request handling benchmarks
fn benchmark_request_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_handling");

    // Test different request sizes
    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload = vec![0u8; size];
            b.iter(|| {
                black_box(process_request(&payload))
            });
        });
    }

    group.finish();
}

// Authentication benchmarks
fn benchmark_authentication(c: &mut Criterion) {
    let mut group = c.benchmark_group("authentication");

    group.bench_function("password_hash", |b| {
        b.iter(|| {
            black_box(hash_password("secure_password_123"))
        });
    });

    group.bench_function("password_verify", |b| {
        let hashed = hash_password("secure_password_123");
        b.iter(|| {
            black_box(verify_password("secure_password_123", &hashed))
        });
    });

    group.bench_function("jwt_encode", |b| {
        b.iter(|| {
            black_box(encode_jwt_token("user123"))
        });
    });

    group.bench_function("jwt_decode", |b| {
        let token = encode_jwt_token("user123");
        b.iter(|| {
            black_box(decode_jwt_token(&token))
        });
    });

    group.finish();
}

// JSON serialization benchmarks
fn benchmark_json_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_operations");

    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestData {
        id: i64,
        name: String,
        email: String,
        active: bool,
        score: f64,
    }

    let test_data = TestData {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        active: true,
        score: 98.5,
    };

    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&test_data).unwrap())
        });
    });

    let json_str = serde_json::to_string(&test_data).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<TestData>(&json_str).unwrap())
        });
    });

    group.finish();
}

// Cache operations benchmarks
fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    let cache: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    group.bench_function("cache_set", |b| {
        let cache = cache.clone();
        b.iter(|| {
            let mut cache = cache.lock().unwrap();
            cache.insert(
                black_box("key".to_string()),
                black_box("value".to_string())
            );
        });
    });

    // Prepopulate cache for get benchmark
    {
        let mut cache = cache.lock().unwrap();
        cache.insert("test_key".to_string(), "test_value".to_string());
    }

    group.bench_function("cache_get", |b| {
        let cache = cache.clone();
        b.iter(|| {
            let cache = cache.lock().unwrap();
            black_box(cache.get("test_key"))
        });
    });

    group.finish();
}

// String operations benchmarks
fn benchmark_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    group.bench_function("concat", |b| {
        b.iter(|| {
            let mut s = String::new();
            for i in 0..100 {
                s.push_str(&i.to_string());
            }
            black_box(s)
        });
    });

    group.bench_function("format", |b| {
        b.iter(|| {
            let mut s = String::new();
            for i in 0..100 {
                s = format!("{}{}", s, i);
            }
            black_box(s)
        });
    });

    group.finish();
}

// Collection operations benchmarks
fn benchmark_collection_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_operations");

    group.bench_function("vec_push", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    group.bench_function("vec_with_capacity", |b| {
        b.iter(|| {
            let mut v = Vec::with_capacity(1000);
            for i in 0..1000 {
                v.push(black_box(i));
            }
            black_box(v)
        });
    });

    let data: Vec<i32> = (0..1000).collect();
    group.bench_function("vec_iter", |b| {
        b.iter(|| {
            let sum: i32 = data.iter().map(|x| x * 2).sum();
            black_box(sum)
        });
    });

    group.finish();
}

// Async operations benchmarks
fn benchmark_async_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_operations");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("spawn_task", |b| {
        b.to_async(&runtime).iter(|| async {
            let handle = tokio::spawn(async {
                black_box(42)
            });
            black_box(handle.await.unwrap())
        });
    });

    group.bench_function("sleep_and_wake", |b| {
        b.to_async(&runtime).iter(|| async {
            tokio::time::sleep(Duration::from_micros(1)).await;
            black_box(())
        });
    });

    group.finish();
}

// Helper functions
fn execute_simple_command() -> i32 {
    42
}

fn execute_complex_command() -> i32 {
    let mut sum = 0;
    for i in 0..100 {
        sum += i;
    }
    sum
}

fn process_request(payload: &[u8]) -> usize {
    payload.len()
}

fn hash_password(password: &str) -> String {
    // Simulate password hashing (simplified for benchmark)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn verify_password(password: &str, hash: &str) -> bool {
    hash_password(password) == hash
}

fn encode_jwt_token(user_id: &str) -> String {
    // Simplified JWT encoding for benchmark
    format!("header.{}.signature", base64::encode(user_id))
}

fn decode_jwt_token(token: &str) -> Option<String> {
    // Simplified JWT decoding for benchmark
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() == 3 {
        base64::decode(parts[1]).ok()
            .and_then(|v| String::from_utf8(v).ok())
    } else {
        None
    }
}

criterion_group!(
    benches,
    benchmark_command_execution,
    benchmark_request_handling,
    benchmark_authentication,
    benchmark_json_operations,
    benchmark_cache_operations,
    benchmark_string_operations,
    benchmark_collection_operations,
    benchmark_async_operations,
);

criterion_main!(benches);
