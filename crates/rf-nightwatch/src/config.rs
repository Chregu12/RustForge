//! Nightwatch configuration

use std::sync::OnceLock;

static CONFIG: OnceLock<NightwatchConfig> = OnceLock::new();

/// Nightwatch configuration
#[derive(Debug, Clone)]
pub struct NightwatchConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Health check interval in seconds
    pub check_interval_secs: u64,
    /// Maximum events to store
    pub max_events: usize,
    /// Enable detailed metrics
    pub detailed_metrics: bool,
    /// Dashboard path prefix
    pub path_prefix: String,
}

impl Default for NightwatchConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 9090,
            check_interval_secs: 60,
            max_events: 10000,
            detailed_metrics: true,
            path_prefix: "/nightwatch".to_string(),
        }
    }
}

impl NightwatchConfig {
    /// Create a new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the host
    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    /// Set the port
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Set the check interval
    pub fn check_interval(mut self, secs: u64) -> Self {
        self.check_interval_secs = secs;
        self
    }

    /// Set maximum events
    pub fn max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// Enable or disable detailed metrics
    pub fn detailed_metrics(mut self, enabled: bool) -> Self {
        self.detailed_metrics = enabled;
        self
    }

    /// Set the path prefix
    pub fn path_prefix(mut self, prefix: &str) -> Self {
        self.path_prefix = prefix.to_string();
        self
    }

    /// Get the bind address
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Set the global configuration
pub fn set_config(config: NightwatchConfig) -> &'static NightwatchConfig {
    CONFIG.get_or_init(|| config)
}

/// Get the global configuration
pub fn get_config() -> &'static NightwatchConfig {
    CONFIG.get_or_init(NightwatchConfig::default)
}
