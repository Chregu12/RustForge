//! Deployment tests for rf-config

#[cfg(test)]
mod tests {
    use rf_config::{AppConfig, ServerConfig, DatabaseConfig, AuthConfig, Config, ConfigLoader};

    // ── AppConfig Struct ─────────────────────────────────────────

    #[test]
    fn app_config_validation_valid() {
        let config = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 8080,
                workers: 4,
                timeout: 30,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/test".into(),
                max_connections: 10,
                min_connections: 1,
                connect_timeout: 5,
            },
            auth: AuthConfig {
                jwt_secret: "a-very-long-and-secure-jwt-secret-key-here".into(),
                token_expiry_hours: 24,
                session_timeout_minutes: 60,
            },
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn app_config_validation_invalid_port() {
        let config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".into(),
                port: 0, // invalid port
                workers: 4,
                timeout: 30,
            },
            database: DatabaseConfig {
                url: "sqlite::memory:".into(),
                max_connections: 5,
                min_connections: 1,
                connect_timeout: 5,
            },
            auth: AuthConfig {
                jwt_secret: "a_sufficiently_long_jwt_secret_key_for_testing".into(),
                token_expiry_hours: 24,
                session_timeout_minutes: 60,
            },
        };
        // Port 0 is invalid — validation should fail
        assert!(config.validate().is_err());
    }

    // ── Config Facade ────────────────────────────────────────────

    #[test]
    fn config_facade_set_get() {
        Config::set("app.name", "RustForge");
        assert_eq!(Config::get("app.name"), Some("RustForge".to_string()));
    }

    #[test]
    fn config_facade_get_or_default() {
        let val = Config::get_or("app.missing", "default_value");
        assert_eq!(val, "default_value");
    }

    #[test]
    fn config_facade_has() {
        Config::set("app.exists", "yes");
        assert!(Config::has("app.exists"));
        assert!(!Config::has("app.definitely_not_set_xyz"));
    }

    #[test]
    fn config_facade_set_value_json() {
        Config::set_value("app.features", serde_json::json!(["auth", "cache", "queue"]));
        let val: Option<Vec<String>> = Config::get_value("app.features");
        assert!(val.is_some());
        assert_eq!(val.unwrap().len(), 3);
    }

    #[test]
    fn config_facade_all() {
        Config::set("test.key1", "val1");
        let all = Config::all();
        assert!(all.contains_key("test.key1"));
    }

    // ── ConfigLoader ─────────────────────────────────────────────

    #[test]
    fn config_loader_creation() {
        let loader = ConfigLoader::new()
            .env("testing")
            .config_dir("config")
            .prefix("RF");
        // Just verifying it builds without panic
        let _ = loader;
    }
}
