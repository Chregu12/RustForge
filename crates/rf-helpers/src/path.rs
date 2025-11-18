//! Path helper functions (storage_path, public_path, etc.)

use std::env;
use std::path::{Path, PathBuf};

/// Get the base path of the application
pub fn base_path(path: Option<&str>) -> PathBuf {
    let base = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    match path {
        Some(p) => base.join(p),
        None => base,
    }
}

/// Get the app directory path
pub fn app_path(path: Option<&str>) -> PathBuf {
    let app = base_path(Some("app"));
    match path {
        Some(p) => app.join(p),
        None => app,
    }
}

/// Get the config directory path
pub fn config_path(path: Option<&str>) -> PathBuf {
    let config = base_path(Some("config"));
    match path {
        Some(p) => config.join(p),
        None => config,
    }
}

/// Get the database directory path
pub fn database_path(path: Option<&str>) -> PathBuf {
    let database = base_path(Some("database"));
    match path {
        Some(p) => database.join(p),
        None => database,
    }
}

/// Get the public directory path
pub fn public_path(path: Option<&str>) -> PathBuf {
    let public = base_path(Some("public"));
    match path {
        Some(p) => public.join(p),
        None => public,
    }
}

/// Get the resources directory path
pub fn resource_path(path: Option<&str>) -> PathBuf {
    let resources = base_path(Some("resources"));
    match path {
        Some(p) => resources.join(p),
        None => resources,
    }
}

/// Get the storage directory path
pub fn storage_path(path: Option<&str>) -> PathBuf {
    let storage = base_path(Some("storage"));
    match path {
        Some(p) => storage.join(p),
        None => storage,
    }
}

/// Get the lang directory path (translations)
pub fn lang_path(path: Option<&str>) -> PathBuf {
    let lang = resource_path(Some("lang"));
    match path {
        Some(p) => lang.join(p),
        None => lang,
    }
}

/// Get the views directory path
pub fn view_path(path: Option<&str>) -> PathBuf {
    let views = resource_path(Some("views"));
    match path {
        Some(p) => views.join(p),
        None => views,
    }
}

/// Get the storage app directory path
pub fn storage_app_path(path: Option<&str>) -> PathBuf {
    let app = storage_path(Some("app"));
    match path {
        Some(p) => app.join(p),
        None => app,
    }
}

/// Get the storage framework directory path
pub fn storage_framework_path(path: Option<&str>) -> PathBuf {
    let framework = storage_path(Some("framework"));
    match path {
        Some(p) => framework.join(p),
        None => framework,
    }
}

/// Get the storage logs directory path
pub fn storage_logs_path(path: Option<&str>) -> PathBuf {
    let logs = storage_path(Some("logs"));
    match path {
        Some(p) => logs.join(p),
        None => logs,
    }
}

/// Mix path helper (for compiled assets)
pub fn mix(path: &str, manifest_directory: Option<&str>) -> String {
    // This would typically read from mix-manifest.json
    // For now, return the path as-is
    let directory = manifest_directory.unwrap_or("");
    format!("{}/{}", directory, path.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_path() {
        let base = base_path(None);
        assert!(base.is_absolute() || base.as_os_str() == ".");

        let with_path = base_path(Some("foo"));
        assert!(with_path.ends_with("foo"));
    }

    #[test]
    fn test_app_path() {
        let app = app_path(None);
        assert!(app.ends_with("app"));

        let with_path = app_path(Some("models"));
        assert!(with_path.ends_with("app/models") || with_path.ends_with("app\\models"));
    }

    #[test]
    fn test_storage_path() {
        let storage = storage_path(None);
        assert!(storage.ends_with("storage"));

        let with_path = storage_path(Some("app/public"));
        assert!(with_path.to_string_lossy().contains("storage"));
        assert!(with_path.to_string_lossy().contains("app"));
    }

    #[test]
    fn test_public_path() {
        let public = public_path(None);
        assert!(public.ends_with("public"));
    }

    #[test]
    fn test_config_path() {
        let config = config_path(None);
        assert!(config.ends_with("config"));
    }

    #[test]
    fn test_database_path() {
        let database = database_path(None);
        assert!(database.ends_with("database"));
    }

    #[test]
    fn test_resource_path() {
        let resources = resource_path(None);
        assert!(resources.ends_with("resources"));
    }

    #[test]
    fn test_storage_logs_path() {
        let logs = storage_logs_path(None);
        assert!(logs.to_string_lossy().contains("storage"));
        assert!(logs.ends_with("logs"));
    }

    #[test]
    fn test_mix() {
        assert_eq!(mix("css/app.css", None), "/css/app.css");
        assert_eq!(mix("/css/app.css", Some("/build")), "/build/css/app.css");
    }
}
