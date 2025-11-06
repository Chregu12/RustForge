use std::sync::Arc;
use foundry_application::FoundryApplication;
use foundry_infra::config::AppConfig;

#[tokio::test]
async fn test_application_bootstrap() {
    // Test that the application can be created and initialized
    let config = AppConfig::default();
    let app = FoundryApplication::new(config);

    assert!(app.is_ok(), "Application should bootstrap successfully");
}

#[tokio::test]
async fn test_application_with_custom_config() {
    // Test application with custom configuration
    let mut config = AppConfig::default();
    config.app_name = "TestApp".to_string();
    config.environment = "testing".to_string();

    let app = FoundryApplication::new(config);
    assert!(app.is_ok(), "Application should accept custom config");
}

#[tokio::test]
async fn test_service_container_initialization() {
    // Test that the service container is properly initialized
    let config = AppConfig::default();
    let app = FoundryApplication::new(config);

    assert!(app.is_ok());
    // Additional service container checks can be added here
}

#[tokio::test]
async fn test_environment_detection() {
    // Test environment detection
    std::env::set_var("APP_ENV", "testing");

    let config = AppConfig::from_env();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.environment, "testing");

    std::env::remove_var("APP_ENV");
}

#[tokio::test]
async fn test_debug_mode() {
    // Test debug mode configuration
    std::env::set_var("APP_DEBUG", "true");

    let config = AppConfig::from_env();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert!(config.debug);

    std::env::remove_var("APP_DEBUG");
}

#[tokio::test]
async fn test_timezone_configuration() {
    // Test timezone configuration
    std::env::set_var("APP_TIMEZONE", "UTC");

    let config = AppConfig::from_env();
    assert!(config.is_ok());

    let config = config.unwrap();
    assert_eq!(config.timezone, "UTC");

    std::env::remove_var("APP_TIMEZONE");
}

#[cfg(test)]
mod helpers {
    use super::*;

    pub fn setup_test_env() {
        std::env::set_var("APP_ENV", "testing");
        std::env::set_var("APP_DEBUG", "true");
        std::env::set_var("DATABASE_URL", "sqlite::memory:");
    }

    pub fn cleanup_test_env() {
        std::env::remove_var("APP_ENV");
        std::env::remove_var("APP_DEBUG");
        std::env::remove_var("DATABASE_URL");
    }
}
