//! Integration tests for rf-config
//!
//! Tests cover: read a value, default when key missing, env-var override,
//! nested keys (database.host style), bool / int / string type variants,
//! AppConfig::from_env defaults, and validation helpers.

use rf_config::{
    facade::Config,
    types::{AppConfig, AuthConfig, DatabaseConfig, ServerConfig},
};
use serde_json::Value;

// ───────────────────────────────────────────────────────────────────────────
// Basic get / set
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn read_a_string_value_that_was_set() {
    Config::set("cfg_test_host", "127.0.0.1");
    let val = Config::get("cfg_test_host");
    assert_eq!(val, Some("127.0.0.1".to_string()));
}

#[test]
fn default_value_returned_when_key_missing() {
    let val = Config::get_or("cfg_totally_missing_key", "default_value");
    assert_eq!(val, "default_value");
}

#[test]
fn has_returns_true_for_existing_key() {
    Config::set("cfg_exists_key", "yes");
    assert!(Config::has("cfg_exists_key"));
}

#[test]
fn has_returns_false_for_missing_key() {
    assert!(!Config::has("cfg_definitely_not_here_xyz123"));
}

// ───────────────────────────────────────────────────────────────────────────
// Typed values
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn typed_integer_value_round_trips() {
    Config::set_value("cfg_int_port", Value::Number(8080.into()));
    let port: Option<u16> = Config::get_value("cfg_int_port");
    assert_eq!(port, Some(8080u16));
}

#[test]
fn typed_bool_value_round_trips() {
    Config::set_value("cfg_bool_flag", Value::Bool(true));
    let flag: Option<bool> = Config::get_value("cfg_bool_flag");
    assert_eq!(flag, Some(true));
}

#[test]
fn typed_default_returned_when_key_absent() {
    let workers: usize = Config::get_value_or("cfg_missing_workers_xyz", 4);
    assert_eq!(workers, 4);
}

// ───────────────────────────────────────────────────────────────────────────
// Nested key convention (dot-separated)
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn nested_key_database_host_can_be_set_and_read() {
    Config::set("database.host", "db.example.com");
    let val = Config::get("database.host");
    assert_eq!(val, Some("db.example.com".to_string()));
}

#[test]
fn nested_key_server_port_stored_as_number() {
    Config::set_value("server.port", Value::Number(9000.into()));
    let port: Option<u16> = Config::get_value("server.port");
    assert_eq!(port, Some(9000u16));
}

// ───────────────────────────────────────────────────────────────────────────
// Config::init mirrors typed config into flat store
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn init_makes_typed_config_accessible_via_typed_api() {
    let app_config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 7777,
            workers: 8,
            timeout: 60,
        },
        database: DatabaseConfig {
            url: "postgres://localhost/test".to_string(),
            max_connections: 15,
            min_connections: 2,
            connect_timeout: 8,
        },
        auth: AuthConfig::default(),
    };

    Config::init(app_config);

    let typed = Config::typed().unwrap();
    assert_eq!(typed.server.port, 7777);
    assert_eq!(typed.server.workers, 8);
}

#[test]
fn init_mirrors_server_port_into_flat_store() {
    let app_config = AppConfig {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 4444,
            workers: 2,
            timeout: 30,
        },
        database: DatabaseConfig::default(),
        auth: AuthConfig::default(),
    };

    Config::init(app_config);

    let port: Option<u16> = Config::get_value("server.port");
    assert_eq!(port, Some(4444u16));
}

// ───────────────────────────────────────────────────────────────────────────
// AppConfig::from_env picks up SERVER_PORT env var
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn from_env_uses_server_port_env_var() {
    std::env::set_var("SERVER_PORT", "5555");
    let config = AppConfig::from_env().expect("should succeed");
    std::env::remove_var("SERVER_PORT");
    assert_eq!(config.server.port, 5555);
}

#[test]
fn from_env_falls_back_to_default_port_when_unset() {
    std::env::remove_var("SERVER_PORT");
    let config = AppConfig::from_env().expect("should succeed");
    // Default port is 3000
    assert_eq!(config.server.port, 3000);
}

// ───────────────────────────────────────────────────────────────────────────
// AppConfig validation
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn validate_passes_for_valid_config() {
    let config = AppConfig {
        server: ServerConfig::default(),
        database: DatabaseConfig::default(),
        auth: AuthConfig::default(),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn validate_fails_for_zero_port() {
    let config = AppConfig {
        server: ServerConfig {
            port: 0,
            ..ServerConfig::default()
        },
        database: DatabaseConfig::default(),
        auth: AuthConfig::default(),
    };
    assert!(config.validate().is_err());
}

#[test]
fn validate_fails_for_zero_workers() {
    let config = AppConfig {
        server: ServerConfig {
            workers: 0,
            ..ServerConfig::default()
        },
        database: DatabaseConfig::default(),
        auth: AuthConfig::default(),
    };
    assert!(config.validate().is_err());
}
