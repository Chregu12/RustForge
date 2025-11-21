//! Database configuration types

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Database configuration
///
/// # Example
///
/// ```rust
/// use rf_orm::DatabaseConfig;
/// use std::time::Duration;
///
/// let config = DatabaseConfig {
///     url: "postgres://localhost/myapp".to_string(),
///     max_connections: 20,
///     min_connections: 5,
///     connect_timeout: Duration::from_secs(8),
///     idle_timeout: Some(Duration::from_secs(600)),
///     acquire_timeout: Duration::from_secs(30),
///     log_queries: true,
///     log_level: "debug".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    ///
    /// Examples:
    /// - SQLite: `sqlite::memory:` or `sqlite://path/to/db.sqlite`
    /// - Postgres: `postgres://user:pass@localhost/dbname`
    /// - MySQL: `mysql://user:pass@localhost/dbname`
    pub url: String,

    /// Maximum number of connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum number of connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout
    #[serde(
        default = "default_connect_timeout",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub connect_timeout: Duration,

    /// Idle connection timeout (None = no timeout)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_option_duration",
        deserialize_with = "deserialize_option_duration"
    )]
    pub idle_timeout: Option<Duration>,

    /// Acquire connection timeout
    #[serde(
        default = "default_acquire_timeout",
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub acquire_timeout: Duration,

    /// Enable SQL query logging
    #[serde(default)]
    pub log_queries: bool,

    /// SQL log level (off, error, warn, info, debug, trace)
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout: default_connect_timeout(),
            idle_timeout: Some(Duration::from_secs(600)),
            acquire_timeout: default_acquire_timeout(),
            log_queries: false,
            log_level: default_log_level(),
        }
    }
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    2
}

fn default_connect_timeout() -> Duration {
    Duration::from_secs(8)
}

fn default_acquire_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_log_level() -> String {
    "info".to_string()
}

// Serde helpers for Duration
fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_secs())
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}

fn serialize_option_duration<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match duration {
        Some(d) => serializer.serialize_some(&d.as_secs()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<u64>::deserialize(deserializer)?;
    Ok(opt.map(Duration::from_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url, "sqlite::memory:");
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 2);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = DatabaseConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 20,
            min_connections: 5,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Some(Duration::from_secs(300)),
            acquire_timeout: Duration::from_secs(15),
            log_queries: true,
            log_level: "debug".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DatabaseConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.url, config.url);
        assert_eq!(deserialized.max_connections, config.max_connections);
        assert_eq!(deserialized.connect_timeout, config.connect_timeout);
        assert_eq!(deserialized.log_queries, config.log_queries);
    }

    #[test]
    fn test_duration_serialization() {
        let config = DatabaseConfig {
            connect_timeout: Duration::from_secs(30),
            ..Default::default()
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["connect_timeout"], 30);
    }
}
