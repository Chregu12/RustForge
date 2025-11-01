//! Individual health checks

use crate::config::FileRequirement;
use crate::report::CheckResult;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use sysinfo::System;

/// Trait for health checks
pub trait HealthCheck {
    /// Run the health check
    async fn run(&self) -> CheckResult;
}

/// Check Rust version
pub struct RustVersionCheck;

impl HealthCheck for RustVersionCheck {
    async fn run(&self) -> CheckResult {
        let version = rustc_version();
        CheckResult::pass("rust", format!("Rust version {}", version))
            .with_details(serde_json::json!({"version": version}))
    }
}

/// Get rustc version
fn rustc_version() -> String {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output();

    match output {
        Ok(output) => {
            let version = String::from_utf8_lossy(&output.stdout);
            version.trim().to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}

/// Check disk space
pub struct DiskSpaceCheck;

impl HealthCheck for DiskSpaceCheck {
    async fn run(&self) -> CheckResult {
        let mut sys = System::new_all();
        sys.refresh_disks();

        let disks = sys.disks();
        if disks.is_empty() {
            return CheckResult::warn("disk", "Could not detect disk information");
        }

        // Get first disk (usually root)
        let disk = &disks[0];
        let available_gb = disk.available_space() / 1024 / 1024 / 1024;
        let total_gb = disk.total_space() / 1024 / 1024 / 1024;

        if available_gb < 1 {
            CheckResult::fail("disk", format!("Low disk space: {} GB available", available_gb))
        } else {
            CheckResult::pass("disk", format!("{} GB / {} GB available", available_gb, total_gb))
                .with_details(serde_json::json!({
                    "available_gb": available_gb,
                    "total_gb": total_gb,
                }))
        }
    }
}

/// Check available memory
pub struct MemoryCheck;

impl HealthCheck for MemoryCheck {
    async fn run(&self) -> CheckResult {
        let mut sys = System::new_all();
        sys.refresh_memory();

        let available_mb = sys.available_memory() / 1024 / 1024;
        let total_mb = sys.total_memory() / 1024 / 1024;

        if available_mb < 256 {
            CheckResult::warn("memory", format!("Low memory: {} MB available", available_mb))
        } else {
            CheckResult::pass("memory", format!("{} MB / {} MB available", available_mb, total_mb))
                .with_details(serde_json::json!({
                    "available_mb": available_mb,
                    "total_mb": total_mb,
                }))
        }
    }
}

/// Check environment variables
pub struct EnvCheck {
    required_vars: Vec<String>,
}

impl EnvCheck {
    pub fn new(required_vars: Vec<String>) -> Self {
        Self { required_vars }
    }
}

impl HealthCheck for EnvCheck {
    async fn run(&self) -> CheckResult {
        if self.required_vars.is_empty() {
            return CheckResult::pass("env", "No required environment variables");
        }

        let mut missing = Vec::new();

        for var in &self.required_vars {
            if env::var(var).is_err() {
                missing.push(var.clone());
            }
        }

        if missing.is_empty() {
            CheckResult::pass(
                "env",
                format!("All {} environment variables set", self.required_vars.len()),
            )
        } else {
            CheckResult::fail(
                "env",
                format!("Missing variables: {}", missing.join(", ")),
            )
            .with_details(serde_json::json!({"missing": missing}))
        }
    }
}

/// Check file permissions
pub struct FilePermissionsCheck {
    files: HashMap<String, FileRequirement>,
}

impl FilePermissionsCheck {
    pub fn new(files: HashMap<String, FileRequirement>) -> Self {
        Self { files }
    }
}

impl HealthCheck for FilePermissionsCheck {
    async fn run(&self) -> CheckResult {
        if self.files.is_empty() {
            return CheckResult::pass("files", "No file requirements");
        }

        let mut issues = Vec::new();

        for (path, req) in &self.files {
            let p = Path::new(path);

            if req.must_exist && !p.exists() {
                issues.push(format!("{} does not exist", path));
                continue;
            }

            if p.exists() {
                if req.readable {
                    if let Err(_) = fs::metadata(p) {
                        issues.push(format!("{} is not readable", path));
                    }
                }

                if req.writable {
                    // Try to open for writing
                    if p.is_file() {
                        if let Err(_) = fs::OpenOptions::new().write(true).open(p) {
                            issues.push(format!("{} is not writable", path));
                        }
                    }
                }
            }
        }

        if issues.is_empty() {
            CheckResult::pass("files", format!("All {} files OK", self.files.len()))
        } else {
            CheckResult::fail("files", format!("Issues: {}", issues.join(", ")))
                .with_details(serde_json::json!({"issues": issues}))
        }
    }
}

/// Check database connectivity
pub struct DatabaseCheck {
    url: String,
}

impl DatabaseCheck {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl HealthCheck for DatabaseCheck {
    async fn run(&self) -> CheckResult {
        // For now, just check if URL is set
        // In a real implementation, this would attempt a connection
        if self.url.is_empty() {
            CheckResult::fail("database", "No database URL configured")
        } else {
            CheckResult::pass("database", "Database URL configured")
                .with_details(serde_json::json!({"url_set": true}))
        }
    }
}

/// Check cache connectivity
pub struct CacheCheck {
    url: String,
}

impl CacheCheck {
    pub fn new(url: String) -> Self {
        Self { url }
    }
}

impl HealthCheck for CacheCheck {
    async fn run(&self) -> CheckResult {
        // For now, just check if URL is set
        // In a real implementation, this would attempt a connection
        if self.url.is_empty() {
            CheckResult::fail("cache", "No cache URL configured")
        } else {
            CheckResult::pass("cache", "Cache URL configured")
                .with_details(serde_json::json!({"url_set": true}))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rust_version_check() {
        let check = RustVersionCheck;
        let result = check.run().await;
        assert_eq!(result.name, "rust");
    }

    #[tokio::test]
    async fn test_disk_space_check() {
        let check = DiskSpaceCheck;
        let result = check.run().await;
        assert_eq!(result.name, "disk");
    }

    #[tokio::test]
    async fn test_memory_check() {
        let check = MemoryCheck;
        let result = check.run().await;
        assert_eq!(result.name, "memory");
    }

    #[tokio::test]
    async fn test_env_check_empty() {
        let check = EnvCheck::new(vec![]);
        let result = check.run().await;
        assert_eq!(result.name, "env");
    }

    #[tokio::test]
    async fn test_env_check_with_vars() {
        env::set_var("TEST_VAR", "test");
        let check = EnvCheck::new(vec!["TEST_VAR".to_string()]);
        let result = check.run().await;
        assert_eq!(result.name, "env");
    }

    #[tokio::test]
    async fn test_file_permissions_check_empty() {
        let check = FilePermissionsCheck::new(HashMap::new());
        let result = check.run().await;
        assert_eq!(result.name, "files");
    }

    #[tokio::test]
    async fn test_database_check() {
        let check = DatabaseCheck::new("postgres://localhost".to_string());
        let result = check.run().await;
        assert_eq!(result.name, "database");
    }

    #[tokio::test]
    async fn test_cache_check() {
        let check = CacheCheck::new("redis://localhost".to_string());
        let result = check.run().await;
        assert_eq!(result.name, "cache");
    }
}
