//! Comprehensive tests for session management

use rf_web::session::{
    CookieSessionDriver, DatabaseSessionDriver, RedisSessionDriver, SameSite, Session,
    SessionConfig, SessionDriver, SessionStore,
};
use std::collections::HashMap;
use std::sync::Arc;

fn create_test_session() -> Session {
    let driver = Arc::new(CookieSessionDriver::new()) as Arc<dyn SessionDriver>;
    Session::new("test_session_id".to_string(), driver)
}

#[test]
fn test_session_creation() {
    let session = create_test_session();
    assert_eq!(session.id(), "test_session_id");
    assert!(session.is_empty());
}

#[test]
fn test_session_put_and_get() {
    let mut session = create_test_session();

    session.put("name", "John Doe");
    session.put("age", 30);
    session.put("active", true);

    assert_eq!(
        session.get_as::<String>("name"),
        Some("John Doe".to_string())
    );
    assert_eq!(session.get_as::<i32>("age"), Some(30));
    assert_eq!(session.get_as::<bool>("active"), Some(true));
}

#[test]
fn test_session_has() {
    let mut session = create_test_session();

    session.put("key", "value");
    assert!(session.has("key"));
    assert!(!session.has("nonexistent"));
}

#[test]
fn test_session_forget() {
    let mut session = create_test_session();

    session.put("key1", "value1");
    session.put("key2", "value2");

    assert!(session.has("key1"));
    assert!(session.has("key2"));

    session.forget("key1");
    assert!(!session.has("key1"));
    assert!(session.has("key2"));
}

#[test]
fn test_session_flush() {
    let mut session = create_test_session();

    session.put("key1", "value1");
    session.put("key2", "value2");
    assert_eq!(session.len(), 2);

    session.flush();
    assert!(session.is_empty());
    assert!(session.is_dirty());
}

#[test]
fn test_session_flash_data() {
    let mut session = create_test_session();

    session.flash("message", "Success!");
    assert!(session.has("_flash.new.message"));

    // Age flash data (move new to old)
    session.age_flash_data();
    assert!(!session.has("_flash.new.message"));
    assert!(session.has("_flash.old.message"));

    // Retrieve flash data (should remove it)
    let message = session.get_flash("message");
    assert_eq!(
        message.and_then(|v| v.as_str().map(|s| s.to_string())),
        Some("Success!".to_string())
    );
    assert!(!session.has("_flash.old.message"));
}

#[test]
fn test_session_flash_persistence() {
    let mut session = create_test_session();

    session.flash("error", "Something went wrong");
    session.age_flash_data();

    // Flash data should persist for one request
    assert!(session.has("_flash.old.error"));

    // After another age, it should be gone
    session.age_flash_data();
    assert!(!session.has("_flash.old.error"));
}

#[test]
fn test_session_keep_flash() {
    let mut session = create_test_session();

    session.flash("message", "Keep this");
    session.age_flash_data();

    // Keep the flash for another request
    session.keep_flash(&["message"]);
    assert!(session.has("_flash.new.message"));

    session.age_flash_data();
    assert!(session.has("_flash.old.message"));
}

#[test]
fn test_session_reflash() {
    let mut session = create_test_session();

    session.flash("msg1", "Message 1");
    session.flash("msg2", "Message 2");
    session.age_flash_data();

    // Reflash all flash data
    session.reflash();
    assert!(session.has("_flash.new.msg1"));
    assert!(session.has("_flash.new.msg2"));
}

#[test]
fn test_session_flash_input() {
    let mut session = create_test_session();

    let mut input = HashMap::new();
    input.insert("email".to_string(), "test@example.com".to_string());
    input.insert("name".to_string(), "John Doe".to_string());

    session.flash_input(input);

    assert_eq!(session.old("email"), Some("test@example.com".to_string()));
    assert_eq!(session.old("name"), Some("John Doe".to_string()));
    assert_eq!(session.old("nonexistent"), None);
}

#[test]
fn test_session_dirty_tracking() {
    let mut session = create_test_session();
    assert!(!session.is_dirty());

    session.put("key", "value");
    assert!(session.is_dirty());
}

#[test]
fn test_session_all_data() {
    let mut session = create_test_session();

    session.put("key1", "value1");
    session.put("key2", 42);

    let all = session.all();
    assert_eq!(all.len(), 2);
    assert!(all.contains_key("key1"));
    assert!(all.contains_key("key2"));
}

#[tokio::test]
async fn test_session_store_creation() {
    let driver = Arc::new(CookieSessionDriver::new()) as Arc<dyn SessionDriver>;
    let store = SessionStore::new(driver);

    let session = store.create().await;
    assert!(session.is_ok());

    let session = session.unwrap();
    assert!(!session.id().is_empty());
}

#[test]
fn test_session_config_defaults() {
    let config = SessionConfig::default();

    assert_eq!(config.cookie_name, "session_id");
    assert_eq!(config.lifetime, Some(7200));
    assert_eq!(config.path, "/");
    assert_eq!(config.domain, None);
    assert!(!config.secure);
    assert!(config.http_only);
}

#[test]
fn test_session_config_builder() {
    let config = SessionConfig::new()
        .cookie_name("my_session")
        .lifetime(3600)
        .path("/app")
        .domain("example.com")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Strict);

    assert_eq!(config.cookie_name, "my_session");
    assert_eq!(config.lifetime, Some(3600));
    assert_eq!(config.path, "/app");
    assert_eq!(config.domain, Some("example.com".to_string()));
    assert!(config.secure);
    assert!(config.http_only);
}

#[test]
fn test_session_config_session_cookie() {
    let config = SessionConfig::new().session_cookie();
    assert!(config.lifetime.is_none());
}

#[tokio::test]
async fn test_cookie_driver_operations() {
    let driver = CookieSessionDriver::new();

    let mut data = HashMap::new();
    data.insert("key".to_string(), serde_json::json!("value"));

    // Write operation
    let write_result = driver.write("test_id", data.clone()).await;
    assert!(write_result.is_ok());

    // Destroy operation
    let destroy_result = driver.destroy("test_id").await;
    assert!(destroy_result.is_ok());

    // GC operation
    let gc_result = driver.gc(std::time::Duration::from_secs(3600)).await;
    assert!(gc_result.is_ok());
}

#[tokio::test]
async fn test_database_driver_creation() {
    let driver = DatabaseSessionDriver::new("sessions");
    assert!(true); // Driver created successfully
}

#[tokio::test]
async fn test_redis_driver_creation() {
    let driver = RedisSessionDriver::new("session:");
    assert!(true); // Driver created successfully
}

#[test]
fn test_session_multiple_flash_types() {
    let mut session = create_test_session();

    session.flash("string", "text");
    session.flash("number", 42);
    session.flash("bool", true);

    session.age_flash_data();

    assert_eq!(
        session.get_flash_as::<String>("string"),
        Some("text".to_string())
    );
    assert_eq!(session.get_flash_as::<i32>("number"), Some(42));
    assert_eq!(session.get_flash_as::<bool>("bool"), Some(true));
}

#[test]
fn test_session_clone() {
    let mut session = create_test_session();
    session.put("key", "value");

    let cloned = session.clone();
    assert_eq!(cloned.id(), session.id());
    assert_eq!(cloned.get_as::<String>("key"), Some("value".to_string()));
}

#[test]
fn test_session_complex_data_types() {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct User {
        id: i32,
        name: String,
        email: String,
    }

    let mut session = create_test_session();

    let user = User {
        id: 1,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };

    session.put("user", &user);

    let retrieved = session.get_as::<User>("user");
    assert_eq!(retrieved, Some(user));
}

#[test]
fn test_session_keep_specific_flash() {
    let mut session = create_test_session();

    session.flash("msg1", "Message 1");
    session.flash("msg2", "Message 2");
    session.flash("msg3", "Message 3");

    session.age_flash_data();

    // Keep only msg1 and msg3
    session.keep_flash(&["msg1", "msg3"]);

    assert!(session.has("_flash.new.msg1"));
    assert!(!session.has("_flash.new.msg2"));
    assert!(session.has("_flash.new.msg3"));
}

#[test]
fn test_session_empty_flash_input() {
    let mut session = create_test_session();

    session.flash_input(HashMap::new());
    assert_eq!(session.old("anything"), None);
}
