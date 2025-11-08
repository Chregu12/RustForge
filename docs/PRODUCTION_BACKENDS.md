# Production Backend Migration Guide

This guide covers migrating from in-memory backends to production-ready Redis backends for Cache and Queue systems.

## Overview

RustForge provides two production-ready backend systems:

1. **Cache System** (`foundry-cache`)
   - In-Memory (development)
   - Redis (production) ✅
   - File-based

2. **Queue System** (`foundry-queue`)
   - In-Memory (development)
   - Redis (production) ✅

## Prerequisites

### Redis Installation

#### Docker (Recommended for Development)

```bash
docker run -d \
  --name rustforge-redis \
  -p 6379:6379 \
  redis:latest
```

#### macOS

```bash
brew install redis
brew services start redis
```

#### Linux (Ubuntu/Debian)

```bash
sudo apt-get install redis-server
sudo systemctl start redis
sudo systemctl enable redis
```

#### Verify Installation

```bash
redis-cli ping
# Should return: PONG
```

## Configuration

### 1. Update .env File

Copy the example configuration:

```bash
cp .env.example .env
```

Update the following settings in `.env`:

```bash
# ============================================================
# REDIS CONFIGURATION
# ============================================================
REDIS_URL=redis://127.0.0.1:6379
REDIS_DB=0
REDIS_PASSWORD=                # Leave empty for local development
REDIS_POOL_SIZE=10
REDIS_CONNECT_TIMEOUT=5

# Separate databases for different purposes (optional)
REDIS_CACHE_DB=1
REDIS_QUEUE_DB=2
REDIS_SESSION_DB=3

# ============================================================
# CACHE SETTINGS
# ============================================================
CACHE_DRIVER=redis            # Change from 'memory' to 'redis'
CACHE_TTL=3600
CACHE_PREFIX=rustforge_cache_

# ============================================================
# QUEUE SETTINGS
# ============================================================
QUEUE_DRIVER=redis            # Change from 'memory' to 'redis'
QUEUE_CONNECTION=redis
QUEUE_TIMEOUT=300
QUEUE_PREFIX=queue:
```

### 2. Production Configuration

For production environments, use separate Redis instances or databases:

```bash
# Production Redis with authentication
REDIS_URL=redis://:your-password@production-redis.example.com:6379
REDIS_PASSWORD=your-secure-password

# Use TLS/SSL for production
REDIS_URL=rediss://production-redis.example.com:6380

# Or use Redis Cluster
REDIS_URL=redis://node1:6379,redis://node2:6379,redis://node3:6379
```

## Cache Migration

### From In-Memory to Redis Cache

#### Before (Development)

```rust
use foundry_cache::prelude::*;

// In-memory cache (data lost on restart)
let cache = MemoryStore::new();
```

#### After (Production)

```rust
use foundry_cache::prelude::*;

// Redis cache (persistent, shared across instances)
let cache = RedisStore::from_env()?;

// Or with explicit configuration
let cache = RedisStore::new("redis://127.0.0.1:6379")?;
```

#### Using CacheManager (Recommended)

```rust
use foundry_cache::prelude::*;

// Automatically selects backend based on CACHE_DRIVER env var
let cache = CacheManager::from_env()?;

// Works the same regardless of backend
cache.set("key", &value, Some(Duration::from_secs(3600))).await?;
let value: Option<String> = cache.get("key").await?;
```

### Cache Features in Redis

```rust
// All cache operations work the same
cache.set("user:1", &user, Some(Duration::from_secs(3600))).await?;
cache.get::<User>("user:1").await?;
cache.forget("user:1").await?;
cache.flush().await?;

// Atomic operations
cache.increment("counter", 1).await?;
cache.decrement("counter", 1).await?;

// Batch operations
cache.set_many(vec![
    ("key1".to_string(), value1, Some(Duration::from_secs(60))),
    ("key2".to_string(), value2, Some(Duration::from_secs(60))),
]).await?;
```

### Cache Performance Tips

1. **Use appropriate TTL values**
   ```rust
   // Short-lived data
   cache.set("session", &data, Some(Duration::from_secs(300))).await?;

   // Long-lived data
   cache.set("config", &data, Some(Duration::from_secs(86400))).await?;
   ```

2. **Use prefixes to organize keys**
   ```bash
   CACHE_PREFIX=myapp_prod_cache:
   ```

3. **Monitor Redis memory usage**
   ```bash
   redis-cli info memory
   ```

## Queue Migration

### From In-Memory to Redis Queue

#### Before (Development)

```rust
use foundry_infra::InMemoryQueue;

// In-memory queue (jobs lost on restart)
let queue = Arc::new(InMemoryQueue::default());
```

#### After (Production)

```rust
use foundry_infra::RedisQueue;

// Redis queue (persistent, distributed)
let queue = Arc::new(RedisQueue::from_env()?);
```

#### Using QueueManager (Recommended)

```rust
use foundry_queue::prelude::*;

// Automatically selects backend based on QUEUE_DRIVER env var
let queue = QueueManager::from_env()?;

// Dispatch jobs
let job = Job::new("send_email")
    .with_payload(json!({
        "to": "user@example.com",
        "subject": "Welcome!"
    }));

queue.dispatch(job).await?;
```

### Queue Features in Redis

#### 1. Delayed Jobs

```rust
let job = Job::new("cleanup")
    .with_delay(Duration::from_secs(3600));  // Execute in 1 hour

queue.dispatch(job).await?;
```

#### 2. Job Priority

```rust
// Higher priority jobs processed first
let urgent = Job::new("urgent_task").with_priority(10);
let normal = Job::new("normal_task").with_priority(5);

queue.dispatch(urgent).await?;
queue.dispatch(normal).await?;
```

#### 3. Multiple Queues

```rust
// Dispatch to different queues
let email_job = Job::new("send_email").on_queue("emails");
let report_job = Job::new("generate_report").on_queue("reports");

queue.dispatch(email_job).await?;
queue.dispatch(report_job).await?;
```

#### 4. Worker Process

```rust
use foundry_queue::prelude::*;
use foundry_queue::worker::WorkerConfig;

// Configure worker
let config = WorkerConfig {
    queues: vec!["emails".to_string(), "reports".to_string()],
    max_retries: 3,
    sleep_duration: Duration::from_secs(1),
    ..Default::default()
};

// Create and run worker
let worker = Worker::with_config(queue.clone(), config);
let stats = worker.run().await?;

println!("Processed: {}, Failed: {}", stats.processed, stats.failed);
```

#### 5. Custom Job Handlers

```rust
use async_trait::async_trait;
use foundry_queue::worker::JobHandler;

struct EmailHandler {
    smtp_config: SmtpConfig,
}

#[async_trait]
impl JobHandler for EmailHandler {
    async fn handle(&self, job: &Job) -> QueueResult<Option<Value>> {
        let to = job.payload["to"].as_str().unwrap();
        let subject = job.payload["subject"].as_str().unwrap();

        // Send email
        self.send_email(to, subject).await?;

        Ok(Some(json!({"sent": true, "to": to})))
    }
}

// Register handler
let mut worker = Worker::new(queue);
worker.register_handler("send_email", EmailHandler {
    smtp_config: SmtpConfig::from_env()
});
```

### Queue Performance Tips

1. **Use multiple workers for high throughput**
   ```bash
   # Start multiple worker processes
   foundry queue:work &
   foundry queue:work &
   foundry queue:work &
   ```

2. **Monitor queue depth**
   ```rust
   let size = queue.size().await?;
   if size > 1000 {
       // Alert or scale workers
   }
   ```

3. **Use appropriate timeouts**
   ```rust
   let job = Job::new("long_task")
       .with_timeout(Duration::from_secs(300));
   ```

## Deployment Checklist

### Development → Staging

- [ ] Update .env with Redis connection details
- [ ] Test cache operations
- [ ] Test queue dispatch and processing
- [ ] Monitor Redis memory usage
- [ ] Test job retry logic
- [ ] Test delayed job execution

### Staging → Production

- [ ] Use dedicated Redis instance
- [ ] Enable Redis authentication
- [ ] Enable TLS/SSL encryption
- [ ] Set up Redis persistence (RDB/AOF)
- [ ] Configure Redis max memory policy
- [ ] Set up Redis monitoring
- [ ] Test failover scenarios
- [ ] Document Redis backup procedures
- [ ] Set up worker auto-scaling
- [ ] Configure alerting for queue depth

## Monitoring

### Redis Health Checks

```bash
# Check Redis status
redis-cli ping

# Get info
redis-cli info

# Monitor commands in real-time
redis-cli monitor

# Check queue depth
redis-cli llen queue:queues:default

# Check cache size
redis-cli dbsize

# Check memory usage
redis-cli info memory
```

### Application Metrics

```rust
// Queue metrics
let queue_size = queue.size().await?;
let failed_jobs = queue.failed_jobs().await?;

// Cache metrics
let stats = cache.stats().await?;
println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
```

## Troubleshooting

### Connection Issues

```rust
// Error: Connection refused
// Solution: Ensure Redis is running
docker ps | grep redis

// Error: Authentication failed
// Solution: Check REDIS_PASSWORD in .env
REDIS_URL=redis://:your-password@localhost:6379
```

### Performance Issues

```bash
# Check slow queries
redis-cli slowlog get 10

# Check connection count
redis-cli client list

# Check memory usage
redis-cli info memory
```

### Queue Stalling

```bash
# Check if workers are running
ps aux | grep queue:work

# Check queue size
redis-cli llen queue:queues:default

# Check delayed jobs
redis-cli zcard queue:delayed

# Manually inspect a job
redis-cli get queue:jobs:{job-id}
```

## Rollback Procedure

If you need to rollback to in-memory backends:

1. Update .env:
   ```bash
   CACHE_DRIVER=memory
   QUEUE_DRIVER=memory
   ```

2. Restart application

3. No code changes required (abstraction handles backend switching)

## Best Practices

### Security

1. **Use authentication in production**
   ```bash
   REDIS_PASSWORD=strong-random-password
   ```

2. **Use TLS/SSL for remote connections**
   ```bash
   REDIS_URL=rediss://production-redis:6380
   ```

3. **Restrict network access**
   ```bash
   # Redis configuration
   bind 127.0.0.1
   protected-mode yes
   ```

### Reliability

1. **Enable Redis persistence**
   ```bash
   # Redis configuration (redis.conf)
   save 900 1
   save 300 10
   save 60 10000
   appendonly yes
   ```

2. **Set up Redis replication**
   ```bash
   # Slave configuration
   slaveof master-redis 6379
   ```

3. **Use Redis Sentinel for high availability**

### Performance

1. **Tune connection pool size**
   ```bash
   REDIS_POOL_SIZE=20  # Adjust based on load
   ```

2. **Use pipelining for batch operations**
   ```rust
   cache.set_many(items).await?;  // Uses Redis pipeline internally
   ```

3. **Monitor and optimize key expiration**
   ```bash
   # Check keys with TTL
   redis-cli ttl your-key
   ```

## Further Reading

- [Redis Official Documentation](https://redis.io/documentation)
- [Redis Best Practices](https://redis.io/topics/best-practices)
- [Queue System README](../crates/foundry-queue/README.md)
- [Cache System README](../crates/foundry-cache/README.md)

## Support

For issues or questions:
- Check the [troubleshooting](#troubleshooting) section
- Review Redis logs: `redis-cli logs`
- Review application logs for connection errors
- Create an issue in the repository
