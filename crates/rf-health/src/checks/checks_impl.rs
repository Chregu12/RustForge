//! Built-in health checks

use crate::checker::{CheckResult, HealthCheck};
use async_trait::async_trait;
use serde_json::json;

/// Always healthy check (for testing)
pub struct AlwaysHealthyCheck {
    name: String,
}

impl AlwaysHealthyCheck {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl HealthCheck for AlwaysHealthyCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> CheckResult {
        CheckResult::healthy(&self.name)
    }
}

/// Memory usage check
pub struct MemoryCheck {
    /// Warning threshold (0.0 - 1.0)
    warning_threshold: f64,
    /// Critical threshold (0.0 - 1.0)
    critical_threshold: f64,
}

impl MemoryCheck {
    /// Create new memory check
    ///
    /// # Arguments
    ///
    /// * `warning_threshold` - Warning threshold (0.0 - 1.0, e.g., 0.8 for 80%)
    /// * `critical_threshold` - Critical threshold (0.0 - 1.0, e.g., 0.95 for 95%)
    pub fn new(warning_threshold: f64, critical_threshold: f64) -> Self {
        Self {
            warning_threshold,
            critical_threshold,
        }
    }

    /// Default thresholds (80% warning, 95% critical)
    pub fn default() -> Self {
        Self::new(0.8, 0.95)
    }
}

#[async_trait]
impl HealthCheck for MemoryCheck {
    fn name(&self) -> &str {
        "memory"
    }

    async fn check(&self) -> CheckResult {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_memory();

        let total = sys.total_memory();
        let used = sys.used_memory();
        let usage = used as f64 / total as f64;

        let result = if usage >= self.critical_threshold {
            CheckResult::unhealthy(
                self.name(),
                format!("Memory usage critical: {:.1}%", usage * 100.0),
            )
        } else if usage >= self.warning_threshold {
            CheckResult::degraded(
                self.name(),
                format!("Memory usage high: {:.1}%", usage * 100.0),
            )
        } else {
            CheckResult::healthy(self.name())
        };

        result
            .with_metadata("total_bytes", json!(total))
            .with_metadata("used_bytes", json!(used))
            .with_metadata("usage_percent", json!(usage * 100.0))
    }

    fn is_liveness(&self) -> bool {
        false
    }

    fn is_readiness(&self) -> bool {
        true
    }
}

/// Disk space check
pub struct DiskCheck {
    /// Path to check
    path: String,
    /// Warning threshold (0.0 - 1.0)
    warning_threshold: f64,
    /// Critical threshold (0.0 - 1.0)
    critical_threshold: f64,
}

impl DiskCheck {
    /// Create new disk check
    ///
    /// # Arguments
    ///
    /// * `path` - Path to check (e.g., "/")
    /// * `warning_threshold` - Warning threshold (0.0 - 1.0, e.g., 0.8 for 80%)
    /// * `critical_threshold` - Critical threshold (0.0 - 1.0, e.g., 0.95 for 95%)
    pub fn new(path: impl Into<String>, warning_threshold: f64, critical_threshold: f64) -> Self {
        Self {
            path: path.into(),
            warning_threshold,
            critical_threshold,
        }
    }

    /// Default check for root (80% warning, 95% critical)
    pub fn default() -> Self {
        Self::new("/", 0.8, 0.95)
    }
}

#[async_trait]
impl HealthCheck for DiskCheck {
    fn name(&self) -> &str {
        "disk"
    }

    async fn check(&self) -> CheckResult {
        use sysinfo::Disks;

        let disks = Disks::new_with_refreshed_list();

        // Find disk containing the path
        let disk = disks.iter().find(|d| {
            self.path
                .starts_with(d.mount_point().to_string_lossy().as_ref())
        });

        match disk {
            Some(disk) => {
                let total = disk.total_space();
                let available = disk.available_space();
                let used = total - available;
                let usage = used as f64 / total as f64;

                let result = if usage >= self.critical_threshold {
                    CheckResult::unhealthy(
                        self.name(),
                        format!("Disk usage critical: {:.1}%", usage * 100.0),
                    )
                } else if usage >= self.warning_threshold {
                    CheckResult::degraded(
                        self.name(),
                        format!("Disk usage high: {:.1}%", usage * 100.0),
                    )
                } else {
                    CheckResult::healthy(self.name())
                };

                result
                    .with_metadata("path", json!(&self.path))
                    .with_metadata("total_bytes", json!(total))
                    .with_metadata("used_bytes", json!(used))
                    .with_metadata("available_bytes", json!(available))
                    .with_metadata("usage_percent", json!(usage * 100.0))
            }
            None => CheckResult::unhealthy(self.name(), format!("Disk not found: {}", self.path)),
        }
    }

    fn is_liveness(&self) -> bool {
        false
    }

    fn is_readiness(&self) -> bool {
        true
    }
}

/// Ping check - verify an external URL/service is reachable
pub struct PingCheck {
    name: String,
    url: String,
    timeout: std::time::Duration,
}

impl PingCheck {
    /// Create a new ping check
    ///
    /// * `name` - Check name (e.g., "api_gateway")
    /// * `url` - URL to check (e.g., "https://api.example.com/health")
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            timeout: std::time::Duration::from_secs(5),
        }
    }

    /// Set the timeout for the check
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl HealthCheck for PingCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self) -> CheckResult {
        let start = std::time::Instant::now();

        match tokio::time::timeout(
            self.timeout,
            async {
                // Use a simple TCP connection check
                let is_https = self.url.starts_with("https://");
                let url = self.url.trim_start_matches("http://").trim_start_matches("https://");
                let host = url.split('/').next().unwrap_or(url);
                let addr = if host.contains(':') {
                    host.to_string()
                } else {
                    let default_port = if is_https { 443 } else { 80 };
                    format!("{}:{}", host, default_port)
                };
                tokio::net::TcpStream::connect(&addr).await
            },
        )
        .await
        {
            Ok(Ok(_)) => {
                let duration = start.elapsed();
                CheckResult::healthy(&self.name)
                    .with_metadata("url", json!(&self.url))
                    .with_metadata("response_time_ms", json!(duration.as_millis()))
            }
            Ok(Err(e)) => CheckResult::unhealthy(
                &self.name,
                format!("Connection failed: {}", e),
            )
            .with_metadata("url", json!(&self.url)),
            Err(_) => CheckResult::unhealthy(
                &self.name,
                format!("Timeout after {}ms", self.timeout.as_millis()),
            )
            .with_metadata("url", json!(&self.url)),
        }
    }

    fn is_readiness(&self) -> bool {
        true
    }
}

/// Uptime check - reports how long the service has been running
pub struct UptimeCheck {
    started_at: std::time::Instant,
}

impl UptimeCheck {
    pub fn new() -> Self {
        Self {
            started_at: std::time::Instant::now(),
        }
    }
}

impl Default for UptimeCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HealthCheck for UptimeCheck {
    fn name(&self) -> &str {
        "uptime"
    }

    async fn check(&self) -> CheckResult {
        let uptime = self.started_at.elapsed();
        CheckResult::healthy(self.name())
            .with_metadata("uptime_seconds", json!(uptime.as_secs()))
            .with_metadata("uptime_human", json!(format_duration(uptime)))
    }

    fn is_liveness(&self) -> bool {
        true
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Database connectivity check (requires "database" feature)
#[cfg(feature = "database")]
pub struct DatabaseCheck {
    pool: sqlx::PgPool,
}

#[cfg(feature = "database")]
impl DatabaseCheck {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "database")]
#[async_trait]
impl HealthCheck for DatabaseCheck {
    fn name(&self) -> &str {
        "database"
    }

    async fn check(&self) -> CheckResult {
        match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => CheckResult::healthy(self.name()),
            Err(e) => CheckResult::unhealthy(self.name(), format!("Database query failed: {}", e)),
        }
    }

    fn is_liveness(&self) -> bool {
        false
    }

    fn is_readiness(&self) -> bool {
        true
    }
}

/// Redis connectivity check (requires "redis-check" feature)
#[cfg(feature = "redis-check")]
pub struct RedisCheck {
    pool: deadpool_redis::Pool,
}

#[cfg(feature = "redis-check")]
impl RedisCheck {
    pub fn new(pool: deadpool_redis::Pool) -> Self {
        Self { pool }
    }

    pub async fn from_url(redis_url: &str) -> Result<Self, crate::error::HealthError> {
        use deadpool_redis::{Config, Runtime};

        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| crate::error::HealthError::RedisError(e.to_string()))?;

        Ok(Self { pool })
    }
}

#[cfg(feature = "redis-check")]
#[async_trait]
impl HealthCheck for RedisCheck {
    fn name(&self) -> &str {
        "redis"
    }

    async fn check(&self) -> CheckResult {
        use redis::AsyncCommands;

        match self.pool.get().await {
            Ok(mut conn) => match redis::cmd("PING").query_async::<_, String>(&mut conn).await {
                Ok(_) => CheckResult::healthy(self.name()),
                Err(e) => CheckResult::unhealthy(self.name(), format!("Redis PING failed: {}", e)),
            },
            Err(e) => {
                CheckResult::unhealthy(self.name(), format!("Redis connection failed: {}", e))
            }
        }
    }

    fn is_liveness(&self) -> bool {
        false
    }

    fn is_readiness(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_always_healthy_check() {
        let check = AlwaysHealthyCheck::new("test");
        let result = check.check().await;

        assert_eq!(result.name, "test");
        assert!(result.status.is_healthy());
    }

    #[tokio::test]
    async fn test_memory_check() {
        let check = MemoryCheck::default();
        let result = check.check().await;

        assert_eq!(result.name, "memory");
        assert!(result.metadata.contains_key("total_bytes"));
        assert!(result.metadata.contains_key("used_bytes"));
        assert!(result.metadata.contains_key("usage_percent"));
    }

    #[tokio::test]
    async fn test_disk_check() {
        let check = DiskCheck::default();
        let result = check.check().await;

        assert_eq!(result.name, "disk");
        // Results may vary by system, just check it runs
    }

    #[tokio::test]
    async fn test_uptime_check() {
        let check = UptimeCheck::new();
        let result = check.check().await;

        assert_eq!(result.name, "uptime");
        assert!(result.status.is_healthy());
        assert!(result.metadata.contains_key("uptime_seconds"));
        assert!(result.metadata.contains_key("uptime_human"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(
            format_duration(std::time::Duration::from_secs(90)),
            "1m 30s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(3661)),
            "1h 1m 1s"
        );
        assert_eq!(
            format_duration(std::time::Duration::from_secs(90061)),
            "1d 1h 1m 1s"
        );
    }
}
