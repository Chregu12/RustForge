//! Asset versioning for cache busting

use std::sync::Arc;

/// Asset version strategy
#[derive(Clone)]
pub enum AssetVersion {
    /// Fixed version string
    Fixed(String),

    /// Dynamic version computed on each request
    Dynamic(Arc<dyn Fn() -> String + Send + Sync>),
}

impl AssetVersion {
    /// Get the current version
    pub fn get(&self) -> String {
        match self {
            AssetVersion::Fixed(v) => v.clone(),
            AssetVersion::Dynamic(f) => f(),
        }
    }

    /// Create a version from a file's modification time
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        AssetVersion::Dynamic(Arc::new(move || {
            std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .map(|t| {
                    format!(
                        "{}",
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    )
                })
                .unwrap_or_else(|_| "1".to_string())
        }))
    }

    /// Create a version from Git commit hash
    pub fn from_git_hash() -> Self {
        AssetVersion::Dynamic(Arc::new(|| {
            std::process::Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }))
    }

    /// Create a version from an environment variable
    pub fn from_env(var: impl Into<String>) -> Self {
        let var = var.into();
        AssetVersion::Dynamic(Arc::new(move || {
            std::env::var(&var).unwrap_or_else(|_| "1".to_string())
        }))
    }
}

impl Default for AssetVersion {
    fn default() -> Self {
        AssetVersion::Fixed("1".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_version() {
        let version = AssetVersion::Fixed("v1.2.3".to_string());
        assert_eq!(version.get(), "v1.2.3");
    }

    #[test]
    fn test_dynamic_version() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let version = AssetVersion::Dynamic(Arc::new(move || {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            format!("v{}", count)
        }));

        // Each call increments the counter
        let v1 = version.get();
        let v2 = version.get();
        assert_eq!(v1, "v0");
        assert_eq!(v2, "v1");
    }

    #[test]
    fn test_from_env() {
        std::env::set_var("TEST_VERSION", "2.0.0");
        let version = AssetVersion::from_env("TEST_VERSION");
        assert_eq!(version.get(), "2.0.0");
        std::env::remove_var("TEST_VERSION");
    }

    #[test]
    fn test_default_version() {
        let version = AssetVersion::default();
        assert_eq!(version.get(), "1");
    }
}
