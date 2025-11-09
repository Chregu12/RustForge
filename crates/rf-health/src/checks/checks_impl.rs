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
        use sysinfo::{Disks};

        let disks = Disks::new_with_refreshed_list();

        // Find disk containing the path
        let disk = disks.iter().find(|d| {
            self.path.starts_with(d.mount_point().to_string_lossy().as_ref())
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
            Err(e) => CheckResult::unhealthy(self.name(), format!("Redis connection failed: {}", e)),
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
}
