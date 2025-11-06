//! Configuration types for observability system

use serde::{Deserialize, Serialize};

/// Complete observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// OpenTelemetry configuration
    pub otel: OtelConfig,

    /// Prometheus configuration
    pub prometheus: PrometheusConfig,

    /// Log level (trace, debug, info, warn, error)
    pub log_level: String,

    /// Enable JSON formatted logs
    pub log_json: bool,

    /// Application service name
    pub service_name: String,

    /// Environment (dev, staging, production)
    pub environment: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            otel: OtelConfig::default(),
            prometheus: PrometheusConfig::default(),
            log_level: "info".to_string(),
            log_json: false,
            service_name: "rustforge".to_string(),
            environment: "development".to_string(),
        }
    }
}

/// OpenTelemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelConfig {
    /// Enable OpenTelemetry
    pub enabled: bool,

    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub endpoint: String,

    /// Use TLS for OTLP
    pub use_tls: bool,

    /// Sample rate for traces (0.0 to 1.0)
    pub sample_rate: f64,

    /// Timeout for exports in seconds
    pub timeout_seconds: u64,

    /// Batch span processor settings
    pub batch_config: BatchConfig,
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "http://localhost:4317".to_string(),
            use_tls: false,
            sample_rate: 1.0,
            timeout_seconds: 10,
            batch_config: BatchConfig::default(),
        }
    }
}

/// Batch span processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum queue size
    pub max_queue_size: usize,

    /// Schedule delay in milliseconds
    pub scheduled_delay_millis: u64,

    /// Maximum export batch size
    pub max_export_batch_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 2048,
            scheduled_delay_millis: 5000,
            max_export_batch_size: 512,
        }
    }
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics
    pub enabled: bool,

    /// Metrics endpoint path
    pub endpoint_path: String,

    /// Include process metrics (CPU, memory, etc.)
    pub include_process_metrics: bool,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint_path: "/metrics".to_string(),
            include_process_metrics: true,
        }
    }
}

impl ObservabilityConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            otel: OtelConfig {
                enabled: std::env::var("OTEL_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                    .unwrap_or_else(|_| "http://localhost:4317".to_string()),
                use_tls: std::env::var("OTEL_EXPORTER_OTLP_TLS")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .unwrap_or(false),
                sample_rate: std::env::var("OTEL_TRACES_SAMPLER_ARG")
                    .unwrap_or_else(|_| "1.0".to_string())
                    .parse()
                    .unwrap_or(1.0),
                ..Default::default()
            },
            prometheus: PrometheusConfig {
                enabled: std::env::var("PROMETHEUS_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                endpoint_path: std::env::var("PROMETHEUS_ENDPOINT")
                    .unwrap_or_else(|_| "/metrics".to_string()),
                ..Default::default()
            },
            log_level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            log_json: std::env::var("LOG_JSON")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "rustforge".to_string()),
            environment: std::env::var("ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.service_name, "rustforge");
        assert_eq!(config.log_level, "info");
        assert!(config.otel.enabled);
    }

    #[test]
    fn test_from_env() {
        std::env::set_var("OTEL_SERVICE_NAME", "test-service");
        std::env::set_var("OTEL_ENABLED", "false");

        let config = ObservabilityConfig::from_env();
        assert_eq!(config.service_name, "test-service");
        assert!(!config.otel.enabled);

        std::env::remove_var("OTEL_SERVICE_NAME");
        std::env::remove_var("OTEL_ENABLED");
    }
}
