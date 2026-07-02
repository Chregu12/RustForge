//! Global authentication manager

use crate::password::PasswordHasher;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock};

/// Global authentication manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_AUTH: Lazy<RwLock<AuthManager>> = Lazy::new(|| {
    RwLock::new(AuthManager::new())
});

/// Resolves login credentials to a stored user record for [`AuthManager::attempt`].
///
/// Implement this on your app's user store (database, in-memory, etc.). Given the
/// submitted `credentials` (e.g. `{"email": ..., "password": ...}`), return the
/// matching user record **including its hashed password** so that `attempt` can
/// verify it, or `None` if no user matches the identifier. `attempt` never trusts
/// the submitted password directly — it always verifies it against the hash in the
/// record returned here.
pub trait UserProvider: Send + Sync {
    /// Look up the stored user record by its login identifier (not the password).
    fn retrieve_by_credentials(&self, credentials: &Value) -> Option<Value>;

    /// Name of the field in the returned record holding the (hashed) password.
    fn password_field(&self) -> &str {
        "password"
    }
}

/// Authentication manager that holds the current authentication state
pub struct AuthManager {
    /// Currently authenticated user (as JSON)
    current_user: Option<Value>,
    /// Remember me state
    via_remember: bool,
    /// Current guard name
    guard: String,
    /// Optional user provider used by [`AuthManager::attempt`] to look up and
    /// verify credentials. When `None`, `attempt` denies every request
    /// (fail-closed) rather than authenticating unverified input.
    provider: Option<Arc<dyn UserProvider>>,
}

impl std::fmt::Debug for AuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManager")
            .field("current_user", &self.current_user)
            .field("via_remember", &self.via_remember)
            .field("guard", &self.guard)
            .field("provider", &self.provider.as_ref().map(|_| "<provider>"))
            .finish()
    }
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        Self {
            current_user: None,
            via_remember: false,
            guard: "web".to_string(),
            provider: None,
        }
    }

    /// Register the [`UserProvider`] that [`attempt`](Self::attempt) uses to look
    /// up and verify credentials. Until one is set, `attempt` fails closed.
    pub fn set_provider(&mut self, provider: Arc<dyn UserProvider>) {
        self.provider = Some(provider);
    }

    /// Check if a user is authenticated
    pub fn check(&self) -> bool {
        self.current_user.is_some()
    }

    /// Check if a user is not authenticated
    pub fn guest(&self) -> bool {
        self.current_user.is_none()
    }

    /// Get the currently authenticated user
    pub fn user<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        self.current_user.as_ref().and_then(|user| {
            serde_json::from_value(user.clone()).ok()
        })
    }

    /// Get the ID of the currently authenticated user
    pub fn id(&self) -> Option<u64> {
        self.current_user.as_ref().and_then(|user| {
            user.get("id")
                .and_then(|id| id.as_u64())
        })
    }

    /// Login a user
    pub fn login<T: Serialize>(&mut self, user: T) -> Result<(), String> {
        let user_json = serde_json::to_value(user)
            .map_err(|e| format!("Failed to serialize user: {}", e))?;

        self.current_user = Some(user_json);
        self.via_remember = false;
        Ok(())
    }

    /// Login a user with remember me
    pub fn login_with_remember<T: Serialize>(&mut self, user: T, remember: bool) -> Result<(), String> {
        self.login(user)?;
        self.via_remember = remember;
        Ok(())
    }

    /// Logout the current user
    pub fn logout(&mut self) {
        self.current_user = None;
        self.via_remember = false;
    }

    /// Attempt to authenticate with the given credentials.
    ///
    /// Looks the user up through the registered [`UserProvider`] and verifies the
    /// submitted password against the stored hash (bcrypt/argon2 auto-detected). On
    /// success the user is logged in (with the password field stripped) and `true`
    /// is returned; on any mismatch `false` is returned.
    ///
    /// This is **fail-closed**: if no provider has been registered, or the user is
    /// not found, or the record has no usable password hash, authentication is
    /// denied. It never authenticates unverified input.
    pub fn attempt(&mut self, credentials: Value) -> Result<bool, String> {
        // No provider configured -> deny (do NOT log anyone in).
        let provider = match &self.provider {
            Some(p) => p.clone(),
            None => return Ok(false),
        };

        let password = match credentials.get("password").and_then(Value::as_str) {
            Some(p) => p,
            None => return Ok(false),
        };

        // Look up the stored record for this identifier.
        let record = match provider.retrieve_by_credentials(&credentials) {
            Some(r) => r,
            None => return Ok(false),
        };

        // Verify the submitted password against the stored hash.
        let hash = match record.get(provider.password_field()).and_then(Value::as_str) {
            Some(h) => h,
            None => return Ok(false),
        };

        let verified = PasswordHasher::default()
            .verify(password, hash)
            .map_err(|e| format!("Password verification failed: {}", e))?;

        if !verified {
            return Ok(false);
        }

        // Log the user in, but never keep the password hash in the session state.
        let mut user = record;
        if let Some(obj) = user.as_object_mut() {
            obj.remove(provider.password_field());
        }
        self.current_user = Some(user);
        self.via_remember = false;
        Ok(true)
    }

    /// Check if the user was authenticated via remember me
    pub fn via_remember(&self) -> bool {
        self.via_remember
    }

    /// Get the current guard name
    pub fn guard_name(&self) -> &str {
        &self.guard
    }

    /// Set the current guard
    pub fn set_guard(&mut self, guard: String) {
        self.guard = guard;
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        if let Some(user) = &self.current_user {
            if let Some(roles) = user.get("roles").and_then(|r| r.as_array()) {
                return roles.iter().any(|r| r.as_str() == Some(role));
            }
        }
        false
    }

    /// Check if user has any of the given roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the given roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        id: u64,
        email: String,
        name: String,
    }

    #[test]
    fn test_auth_manager_new() {
        let manager = AuthManager::new();
        assert!(!manager.check());
        assert!(manager.guest());
    }

    #[test]
    fn test_auth_manager_login() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user.clone()).unwrap();
        assert!(manager.check());
        assert!(!manager.guest());

        let current: Option<TestUser> = manager.user();
        assert_eq!(current, Some(user));
    }

    #[test]
    fn test_auth_manager_logout() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user).unwrap();
        assert!(manager.check());

        manager.logout();
        assert!(!manager.check());
        assert!(manager.guest());
    }

    #[test]
    fn test_auth_manager_id() {
        let mut manager = AuthManager::new();
        assert_eq!(manager.id(), None);

        let user = TestUser {
            id: 42,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user).unwrap();
        assert_eq!(manager.id(), Some(42));
    }

    #[test]
    fn test_auth_manager_via_remember() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login_with_remember(user, true).unwrap();
        assert!(manager.via_remember());
    }

    /// Test provider backing a single user with a real bcrypt-hashed password.
    struct TestProvider {
        email: String,
        password_hash: String,
    }

    impl UserProvider for TestProvider {
        fn retrieve_by_credentials(&self, credentials: &Value) -> Option<Value> {
            let email = credentials.get("email").and_then(Value::as_str)?;
            if email == self.email {
                Some(serde_json::json!({
                    "id": 1,
                    "email": self.email,
                    "name": "Real User",
                    "password": self.password_hash,
                }))
            } else {
                None
            }
        }
    }

    fn provider() -> Arc<dyn UserProvider> {
        let hash = PasswordHasher::bcrypt(4).unwrap().hash("secret").unwrap();
        Arc::new(TestProvider {
            email: "test@example.com".to_string(),
            password_hash: hash,
        })
    }

    #[test]
    fn test_attempt_fails_closed_without_provider() {
        let mut manager = AuthManager::new();
        let credentials = serde_json::json!({"email": "test@example.com", "password": "secret"});
        assert!(!manager.attempt(credentials).unwrap());
        assert!(!manager.check());
    }

    #[test]
    fn test_attempt_succeeds_with_correct_password() {
        let mut manager = AuthManager::new();
        manager.set_provider(provider());
        let credentials = serde_json::json!({"email": "test@example.com", "password": "secret"});
        assert!(manager.attempt(credentials).unwrap());
        assert!(manager.check());
        // Password hash must never be exposed in the session state.
        assert!(manager.user::<Value>().unwrap().get("password").is_none());
    }

    #[test]
    fn test_attempt_rejects_wrong_password_and_unknown_user() {
        let mut manager = AuthManager::new();
        manager.set_provider(provider());

        let wrong_pw = serde_json::json!({"email": "test@example.com", "password": "nope"});
        assert!(!manager.attempt(wrong_pw).unwrap());
        assert!(!manager.check());

        let unknown = serde_json::json!({"email": "ghost@example.com", "password": "secret"});
        assert!(!manager.attempt(unknown).unwrap());
        assert!(!manager.check());
    }

    #[test]
    fn test_auth_manager_guard() {
        let mut manager = AuthManager::new();
        assert_eq!(manager.guard_name(), "web");

        manager.set_guard("api".to_string());
        assert_eq!(manager.guard_name(), "api");
    }
}
