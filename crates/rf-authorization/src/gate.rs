use crate::error::{AuthorizationError, AuthorizationResult};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A gate callback function
type GateCallback<U> = Arc<dyn Fn(&U) -> bool + Send + Sync>;

/// Global gate registry
static GATE_REGISTRY: Lazy<Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

use std::any::Any;

/// Simple closure-based authorization gates
///
/// # Example
///
/// ```rust,ignore
/// use rf_authorization::Gate;
///
/// // Define gates
/// Gate::define("edit-settings", |user: &User| user.is_admin());
/// Gate::define("view-dashboard", |user: &User| user.has_role("viewer"));
///
/// // Check authorization
/// # fn example(user: User) -> Result<(), Box<dyn std::error::Error>> {
/// if Gate::allows("edit-settings", &user) {
///     // User can edit settings
/// }
///
/// if Gate::denies("view-dashboard", &user) {
///     // User cannot view dashboard
/// }
///
/// // Authorize or throw error
/// Gate::authorize("edit-settings", &user)?;
/// # Ok(())
/// # }
/// ```
pub struct Gate;

impl Gate {
    /// Define a new gate
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::Gate;
    ///
    /// Gate::define("edit-settings", |user: &User| {
    ///     user.is_admin()
    /// });
    ///
    /// Gate::define("publish-post", |user: &User| {
    ///     user.has_permission("posts.publish")
    /// });
    /// ```
    pub fn define<F, U: 'static>(name: impl Into<String>, callback: F)
    where
        F: Fn(&U) -> bool + Send + Sync + 'static,
    {
        let name = name.into();
        let callback: GateCallback<U> = Arc::new(callback);

        let mut registry = GATE_REGISTRY.write().unwrap();
        registry.insert(name.clone(), Box::new(callback));

        tracing::debug!("Registered gate: {}", name);
    }

    /// Check if a gate allows an action
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::Gate;
    ///
    /// # fn example(user: User) -> Result<(), Box<dyn std::error::Error>> {
    /// if Gate::allows("edit-settings", &user) {
    ///     // Show settings UI
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn allows<U: 'static>(name: &str, user: &U) -> bool {
        Self::check(name, user).unwrap_or(false)
    }

    /// Check if a gate denies an action
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::Gate;
    ///
    /// # fn example(user: User) -> Result<(), Box<dyn std::error::Error>> {
    /// if Gate::denies("delete-user", &user) {
    ///     return Err("Not authorized".into());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn denies<U: 'static>(name: &str, user: &U) -> bool {
        !Self::allows(name, user)
    }

    /// Authorize or throw an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::Gate;
    ///
    /// # fn example(user: User) -> Result<(), Box<dyn std::error::Error>> {
    /// Gate::authorize("edit-settings", &user)?;
    /// // If we get here, user is authorized
    /// # Ok(())
    /// # }
    /// ```
    pub fn authorize<U: 'static>(name: &str, user: &U) -> AuthorizationResult<()> {
        if Self::allows(name, user) {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "Gate '{}' denied",
                name
            )))
        }
    }

    /// Internal check method
    fn check<U: 'static>(name: &str, user: &U) -> AuthorizationResult<bool> {
        let registry = GATE_REGISTRY.read().unwrap();

        let callback_box = registry
            .get(name)
            .ok_or_else(|| AuthorizationError::GateNotFound(name.to_string()))?;

        let callback = callback_box
            .downcast_ref::<GateCallback<U>>()
            .ok_or_else(|| AuthorizationError::GateNotFound("Type mismatch".to_string()))?;

        Ok(callback(user))
    }

    /// Check if a gate is defined
    pub fn has(name: &str) -> bool {
        let registry = GATE_REGISTRY.read().unwrap();
        registry.contains_key(name)
    }

    /// Remove a gate
    pub fn forget(name: &str) {
        let mut registry = GATE_REGISTRY.write().unwrap();
        registry.remove(name);
    }

    /// Get all defined gate names
    pub fn all() -> Vec<String> {
        let registry = GATE_REGISTRY.read().unwrap();
        registry.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestUser {
        is_admin: bool,
        role: String,
    }

    impl TestUser {
        fn is_admin(&self) -> bool {
            self.is_admin
        }

        fn has_role(&self, role: &str) -> bool {
            self.role == role
        }
    }

    #[test]
    fn test_gate_definition() {
        Gate::define("test-gate", |user: &TestUser| user.is_admin());

        assert!(Gate::has("test-gate"));
    }

    #[test]
    fn test_gate_allows() {
        Gate::define("admin-only", |user: &TestUser| user.is_admin());

        let admin = TestUser {
            is_admin: true,
            role: "admin".to_string(),
        };

        let regular = TestUser {
            is_admin: false,
            role: "user".to_string(),
        };

        assert!(Gate::allows("admin-only", &admin));
        assert!(!Gate::allows("admin-only", &regular));
    }

    #[test]
    fn test_gate_denies() {
        Gate::define("user-only", |user: &TestUser| !user.is_admin());

        let admin = TestUser {
            is_admin: true,
            role: "admin".to_string(),
        };

        assert!(Gate::denies("user-only", &admin));
    }

    #[test]
    fn test_gate_authorize() {
        Gate::define("viewer-access", |user: &TestUser| user.has_role("viewer"));

        let viewer = TestUser {
            is_admin: false,
            role: "viewer".to_string(),
        };

        let editor = TestUser {
            is_admin: false,
            role: "editor".to_string(),
        };

        assert!(Gate::authorize("viewer-access", &viewer).is_ok());
        assert!(Gate::authorize("viewer-access", &editor).is_err());
    }

    #[test]
    fn test_gate_all() {
        Gate::define("gate1", |_: &TestUser| true);
        Gate::define("gate2", |_: &TestUser| false);

        let gates = Gate::all();
        assert!(gates.len() >= 2);
    }

    #[test]
    fn test_gate_forget() {
        Gate::define("temporary-gate", |_: &TestUser| true);
        assert!(Gate::has("temporary-gate"));

        Gate::forget("temporary-gate");
        assert!(!Gate::has("temporary-gate"));
    }
}
