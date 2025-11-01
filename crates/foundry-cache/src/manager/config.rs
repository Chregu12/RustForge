use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub driver: CacheDriver,
    pub default_ttl: Duration,
    pub prefix: String,
    pub redis: Option<RedisConfig>,
    pub file: Option<FileConfig>,
    pub memory: Option<MemoryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheDriver {
    Redis,
    File,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_capacity: u64,
}

impl CacheConfig {
    pub fn from_env() -> Self {
        let driver = match std::env::var("CACHE_DRIVER")
            .unwrap_or_else(|_| "memory".to_string())
            .to_lowercase()
            .as_str()
        {
            "redis" => CacheDriver::Redis,
            "file" => CacheDriver::File,
            _ => CacheDriver::Memory,
        };

        let default_ttl = std::env::var("CACHE_TTL")
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(3600));

        let prefix = std::env::var("CACHE_PREFIX").unwrap_or_else(|_| "cache:".to_string());

        let redis = if matches!(driver, CacheDriver::Redis) {
            Some(RedisConfig {
                url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
                pool_size: std::env::var("REDIS_POOL_SIZE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
            })
        } else {
            None
        };

        let file = if matches!(driver, CacheDriver::File) {
            Some(FileConfig {
                path: std::env::var("CACHE_FILE_PATH")
                    .unwrap_or_else(|_| "./storage/cache".to_string()),
            })
        } else {
            None
        };

        let memory = if matches!(driver, CacheDriver::Memory) {
            Some(MemoryConfig {
                max_capacity: std::env::var("CACHE_MEMORY_CAPACITY")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10_000),
            })
        } else {
            None
        };

        Self {
            driver,
            default_ttl,
            prefix,
            redis,
            file,
            memory,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
