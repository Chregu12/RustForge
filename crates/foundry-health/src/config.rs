//! Health check configuration

use std::collections::HashMap;

/// Configuration for health checks
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Required environment variables to check
    pub required_env_vars: Vec<String>,
    /// Required files and their permissions
    pub required_files: HashMap<String, FileRequirement>,
    /// Database URL to check connectivity
    pub database_url: Option<String>,
    /// Cache URL to check connectivity
    pub cache_url: Option<String>,
    /// Minimum required Rust version
    pub min_rust_version: Option<String>,
    /// Minimum disk space in MB
    pub min_disk_space_mb: u64,
    /// Minimum available memory in MB
    pub min_memory_mb: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            required_env_vars: Vec::new(),
            required_files: HashMap::new(),
            database_url: None,
            cache_url: None,
            min_rust_version: Some("1.70.0".to_string()),
            min_disk_space_mb: 1024, // 1 GB
            min_memory_mb: 512,       // 512 MB
        }
    }
}

/// File requirement for permission checks
#[derive(Debug, Clone)]
pub struct FileRequirement {
    /// Whether file must exist
    pub must_exist: bool,
    /// Whether file must be readable
    pub readable: bool,
    /// Whether file must be writable
    pub writable: bool,
}

impl FileRequirement {
    /// Create a requirement for an existing readable file
    pub fn readable() -> Self {
        Self {
            must_exist: true,
            readable: true,
            writable: false,
        }
    }

    /// Create a requirement for an existing writable file
    pub fn writable() -> Self {
        Self {
            must_exist: true,
            readable: true,
            writable: true,
        }
    }

    /// Create a requirement for any file (may not exist)
    pub fn optional() -> Self {
        Self {
            must_exist: false,
            readable: false,
            writable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HealthCheckConfig::default();
        assert_eq!(config.min_disk_space_mb, 1024);
        assert_eq!(config.min_memory_mb, 512);
        assert!(config.min_rust_version.is_some());
    }

    #[test]
    fn test_file_requirement_readable() {
        let req = FileRequirement::readable();
        assert!(req.must_exist);
        assert!(req.readable);
        assert!(!req.writable);
    }

    #[test]
    fn test_file_requirement_writable() {
        let req = FileRequirement::writable();
        assert!(req.must_exist);
        assert!(req.readable);
        assert!(req.writable);
    }

    #[test]
    fn test_file_requirement_optional() {
        let req = FileRequirement::optional();
        assert!(!req.must_exist);
        assert!(!req.readable);
        assert!(!req.writable);
    }
}
