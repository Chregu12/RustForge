//! Enhanced Gates - Simple Permission Checks with Callbacks
//!
//! Gates provide a simple, closure-based approach to authorization.
//! They're perfect for simple permission checks that don't need a full policy.

use crate::error::{AuthorizationError, AuthorizationResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A gate callback function that takes a user and ability name
pub type GateCallback<U> = Arc<dyn Fn(&U, &str) -> bool + Send + Sync>;

/// Gate manager for simple permission checks
///
/// # Example
///
/// ```rust
/// use rf_authorization::gates::{Gate, GateCallback};
/// use std::sync::Arc;
///
/// #[derive(Clone)]
/// struct User {
///     id: i64,
///     is_admin: bool,
///     permissions: Vec<String>,
/// }
///
/// impl User {
///     fn has_permission(&self, permission: &str) -> bool {
///         self.permissions.contains(&permission.to_string())
///     }
/// }
///
/// let mut gate = Gate::new();
///
/// // Define a simple gate
/// gate.define("create-post", Arc::new(|user: &User, _| {
///     user.is_admin || user.has_permission("create-post")
/// }));
///
/// gate.define("delete-post", Arc::new(|user: &User, _| {
///     user.is_admin
/// }));
///
/// let admin_user = User {
///     id: 1,
///     is_admin: true,
///     permissions: vec![],
/// };
///
/// let regular_user = User {
///     id: 2,
///     is_admin: false,
///     permissions: vec!["create-post".to_string()],
/// };
///
/// // Check permissions
/// assert!(gate.allows(&admin_user, "create-post"));
/// assert!(gate.allows(&admin_user, "delete-post"));
/// assert!(gate.allows(&regular_user, "create-post"));
/// assert!(gate.denies(&regular_user, "delete-post"));
///
/// // Or throw error
/// assert!(gate.authorize(&admin_user, "delete-post").is_ok());
/// assert!(gate.authorize(&regular_user, "delete-post").is_err());
/// ```
pub struct Gate<U> {
    abilities: Arc<Mutex<HashMap<String, GateCallback<U>>>>,
}

impl<U> Gate<U> {
    /// Create a new Gate instance
    pub fn new() -> Self {
        Self {
            abilities: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Define a new gate ability
    ///
    /// # Arguments
    ///
    /// * `ability` - The name of the ability (e.g., "create-post")
    /// * `callback` - A function that takes a user and ability name, returns bool
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::gates::Gate;
    /// # use std::sync::Arc;
    /// # struct User { is_admin: bool }
    /// let mut gate = Gate::new();
    ///
    /// gate.define("edit-settings", Arc::new(|user: &User, _| {
    ///     user.is_admin
    /// }));
    /// ```
    pub fn define(&mut self, ability: impl Into<String>, callback: GateCallback<U>) {
        let mut abilities = self.abilities.lock().unwrap();
        abilities.insert(ability.into(), callback);
    }

    /// Check if a user is allowed to perform an ability
    ///
    /// Returns `true` if allowed, `false` if denied or ability not found.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::gates::Gate;
    /// # use std::sync::Arc;
    /// # struct User { is_admin: bool }
    /// # let mut gate = Gate::new();
    /// # gate.define("create-post", Arc::new(|u: &User, _| u.is_admin));
    /// # let user = User { is_admin: true };
    /// if gate.allows(&user, "create-post") {
    ///     // User can create post
    /// }
    /// ```
    pub fn allows(&self, user: &U, ability: &str) -> bool {
        let abilities = self.abilities.lock().unwrap();

        if let Some(callback) = abilities.get(ability) {
            callback(user, ability)
        } else {
            false // Deny by default if ability not found
        }
    }

    /// Check if a user is denied from performing an ability
    ///
    /// Returns `true` if denied, `false` if allowed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::gates::Gate;
    /// # use std::sync::Arc;
    /// # struct User { is_admin: bool }
    /// # let mut gate = Gate::new();
    /// # gate.define("delete-user", Arc::new(|u: &User, _| u.is_admin));
    /// # let user = User { is_admin: false };
    /// if gate.denies(&user, "delete-user") {
    ///     // User cannot delete
    /// }
    /// ```
    pub fn denies(&self, user: &U, ability: &str) -> bool {
        !self.allows(user, ability)
    }

    /// Authorize a user or return an error
    ///
    /// # Errors
    ///
    /// Returns `AuthorizationError::Forbidden` if the user is not authorized.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::gates::Gate;
    /// # use std::sync::Arc;
    /// # struct User { is_admin: bool }
    /// # let mut gate = Gate::new();
    /// # gate.define("delete-post", Arc::new(|u: &User, _| u.is_admin));
    /// # let user = User { is_admin: true };
    /// gate.authorize(&user, "delete-post")?;
    /// // If we get here, user is authorized
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn authorize(&self, user: &U, ability: &str) -> AuthorizationResult<()> {
        if self.allows(user, ability) {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "User not authorized for ability: {}", ability
            )))
        }
    }

    /// Check if an ability is defined
    pub fn has(&self, ability: &str) -> bool {
        let abilities = self.abilities.lock().unwrap();
        abilities.contains_key(ability)
    }

    /// Remove an ability
    pub fn forget(&mut self, ability: &str) {
        let mut abilities = self.abilities.lock().unwrap();
        abilities.remove(ability);
    }

    /// Get all defined ability names
    pub fn all(&self) -> Vec<String> {
        let abilities = self.abilities.lock().unwrap();
        abilities.keys().cloned().collect()
    }

    /// Define multiple abilities at once
    pub fn define_many(&mut self, definitions: Vec<(&str, GateCallback<U>)>) {
        let mut abilities = self.abilities.lock().unwrap();
        for (name, callback) in definitions {
            abilities.insert(name.to_string(), callback);
        }
    }

    /// Check multiple abilities (user must have ALL)
    pub fn allows_all(&self, user: &U, abilities: &[&str]) -> bool {
        abilities.iter().all(|ability| self.allows(user, ability))
    }

    /// Check multiple abilities (user must have ANY)
    pub fn allows_any(&self, user: &U, abilities: &[&str]) -> bool {
        abilities.iter().any(|ability| self.allows(user, ability))
    }
}

impl<U> Default for Gate<U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<U> Clone for Gate<U> {
    fn clone(&self) -> Self {
        Self {
            abilities: Arc::clone(&self.abilities),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestUser {
        id: i64,
        is_admin: bool,
        permissions: Vec<String>,
    }

    impl TestUser {
        fn has_permission(&self, permission: &str) -> bool {
            self.permissions.contains(&permission.to_string())
        }
    }

    #[test]
    fn test_gate_allows() {
        let mut gate = Gate::new();
        gate.define("create-post", Arc::new(|user: &TestUser, _| user.is_admin));

        let admin = TestUser {
            id: 1,
            is_admin: true,
            permissions: vec![],
        };
        let regular = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec![],
        };

        assert!(gate.allows(&admin, "create-post"));
        assert!(!gate.allows(&regular, "create-post"));
    }

    #[test]
    fn test_gate_denies() {
        let mut gate = Gate::new();
        gate.define("delete-post", Arc::new(|user: &TestUser, _| user.is_admin));

        let regular = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec![],
        };

        assert!(gate.denies(&regular, "delete-post"));
    }

    #[test]
    fn test_gate_authorize() {
        let mut gate = Gate::new();
        gate.define("view-dashboard", Arc::new(|user: &TestUser, _| {
            user.has_permission("view-dashboard")
        }));

        let user_with_permission = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["view-dashboard".to_string()],
        };
        let user_without_permission = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec![],
        };

        assert!(gate.authorize(&user_with_permission, "view-dashboard").is_ok());
        assert!(gate.authorize(&user_without_permission, "view-dashboard").is_err());
    }

    #[test]
    fn test_gate_default_deny() {
        let gate: Gate<TestUser> = Gate::new();

        let user = TestUser {
            id: 1,
            is_admin: true,
            permissions: vec![],
        };

        // Non-existent ability should deny by default
        assert!(!gate.allows(&user, "undefined-ability"));
        assert!(gate.denies(&user, "undefined-ability"));
    }

    #[test]
    fn test_gate_has() {
        let mut gate = Gate::new();
        gate.define("test-ability", Arc::new(|_: &TestUser, _| true));

        assert!(gate.has("test-ability"));
        assert!(!gate.has("non-existent"));
    }

    #[test]
    fn test_gate_forget() {
        let mut gate = Gate::new();
        gate.define("temporary", Arc::new(|_: &TestUser, _| true));

        assert!(gate.has("temporary"));
        gate.forget("temporary");
        assert!(!gate.has("temporary"));
    }

    #[test]
    fn test_gate_all() {
        let mut gate = Gate::new();
        gate.define("ability1", Arc::new(|_: &TestUser, _| true));
        gate.define("ability2", Arc::new(|_: &TestUser, _| true));

        let abilities = gate.all();
        assert!(abilities.len() >= 2);
        assert!(abilities.contains(&"ability1".to_string()));
        assert!(abilities.contains(&"ability2".to_string()));
    }

    #[test]
    fn test_gate_define_many() {
        let mut gate = Gate::new();

        gate.define_many(vec![
            ("ability1", Arc::new(|_: &TestUser, _| true)),
            ("ability2", Arc::new(|_: &TestUser, _| false)),
        ]);

        assert!(gate.has("ability1"));
        assert!(gate.has("ability2"));
    }

    #[test]
    fn test_gate_allows_all() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let user_with_all = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["read".to_string(), "write".to_string()],
        };
        let user_with_one = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec!["read".to_string()],
        };

        assert!(gate.allows_all(&user_with_all, &["read", "write"]));
        assert!(!gate.allows_all(&user_with_one, &["read", "write"]));
    }

    #[test]
    fn test_gate_allows_any() {
        let mut gate = Gate::new();
        gate.define("read", Arc::new(|user: &TestUser, _| {
            user.has_permission("read")
        }));
        gate.define("write", Arc::new(|user: &TestUser, _| {
            user.has_permission("write")
        }));

        let user_with_one = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["read".to_string()],
        };
        let user_with_none = TestUser {
            id: 2,
            is_admin: false,
            permissions: vec![],
        };

        assert!(gate.allows_any(&user_with_one, &["read", "write"]));
        assert!(!gate.allows_any(&user_with_none, &["read", "write"]));
    }

    #[test]
    fn test_gate_clone() {
        let mut gate = Gate::new();
        gate.define("test", Arc::new(|_: &TestUser, _| true));

        let cloned = gate.clone();
        assert!(cloned.has("test"));
    }

    #[test]
    fn test_gate_with_ability_parameter() {
        let mut gate = Gate::new();

        // Callback that uses the ability parameter
        gate.define("dynamic", Arc::new(|user: &TestUser, ability: &str| {
            user.has_permission(ability)
        }));

        let user = TestUser {
            id: 1,
            is_admin: false,
            permissions: vec!["dynamic".to_string()],
        };

        assert!(gate.allows(&user, "dynamic"));
    }
}
