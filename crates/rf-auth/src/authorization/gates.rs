//! Gate-based authorization for simple ability checks

use super::error::{AuthorizationError, AuthorizationResult};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

/// A gate check function that takes a user and returns whether they're authorized
///
/// The function must return a future that resolves to a boolean.
pub type GateCheck<U> = Arc<dyn Fn(&U) -> BoxFuture<'static, bool> + Send + Sync>;

/// Gate system for simple ability-based authorization
///
/// Gates are simple closures that determine if a user can perform a named ability.
/// Unlike policies, gates don't operate on specific resources, but rather check
/// general abilities.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::gates::Gate;
///
/// # async fn example() {
/// #[derive(Clone)]
/// struct User {
///     role: String,
/// }
///
/// // Define a gate
/// let gate: Gate<User> = Gate::new();
/// gate.define("admin", |user: &User| {
///     let role = user.role.clone();
///     async move { role == "admin" }
/// });
///
/// let admin = User { role: "admin".to_string() };
/// let user = User { role: "user".to_string() };
///
/// assert!(gate.allows(&admin, "admin").await);
/// assert!(!gate.allows(&user, "admin").await);
/// # }
/// ```
pub struct Gate<U = ()> {
    checks: Arc<Mutex<HashMap<String, GateCheck<U>>>>,
}

impl<U> Gate<U>
where
    U: Send + Sync + 'static,
{
    /// Create a new Gate instance
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Define a new gate with a check function
    ///
    /// # Arguments
    ///
    /// - `name`: The name of the gate
    /// - `check`: A function that takes a user reference and returns a future that resolves to bool
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::gates::Gate;
    ///
    /// struct User { role: String }
    ///
    /// let gate: Gate<User> = Gate::new();
    ///
    /// gate.define("admin", |user: &User| {
    ///     let role = user.role.clone();
    ///     async move { role == "admin" }
    /// });
    ///
    /// gate.define("can-publish", |user: &User| {
    ///     let role = user.role.clone();
    ///     async move { role == "admin" || role == "editor" }
    /// });
    /// ```
    pub fn define<F, Fut>(&self, name: &str, check: F)
    where
        F: Fn(&U) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let check: GateCheck<U> = Arc::new(move |user| Box::pin(check(user)));

        let mut checks = self.checks.lock().unwrap();
        checks.insert(name.to_string(), check);
    }

    /// Check if a user is allowed by a gate
    ///
    /// Returns `true` if the gate allows the user, `false` if it denies them,
    /// or `false` if the gate doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::gates::Gate;
    ///
    /// # async fn example() {
    /// struct User { role: String }
    ///
    /// let gate: Gate<User> = Gate::new();
    /// gate.define("admin", |user: &User| {
    ///     let role = user.role.clone();
    ///     async move { role == "admin" }
    /// });
    ///
    /// let admin = User { role: "admin".to_string() };
    /// let user = User { role: "user".to_string() };
    ///
    /// assert!(gate.allows(&admin, "admin").await);
    /// assert!(!gate.allows(&user, "admin").await);
    /// # }
    /// ```
    pub async fn allows(&self, user: &U, gate: &str) -> bool {
        let check = {
            let checks = self.checks.lock().unwrap();
            checks.get(gate).cloned()
        };

        match check {
            Some(check) => check(user).await,
            None => false,
        }
    }

    /// Check if a user is denied by a gate
    ///
    /// This is the inverse of `allows`.
    pub async fn denies(&self, user: &U, gate: &str) -> bool {
        !self.allows(user, gate).await
    }

    /// Check if a user is allowed by any of the given gates
    ///
    /// Returns `true` if at least one gate allows the user.
    pub async fn any(&self, user: &U, gates: &[&str]) -> bool {
        for gate in gates {
            if self.allows(user, gate).await {
                return true;
            }
        }
        false
    }

    /// Check if a user is allowed by all of the given gates
    ///
    /// Returns `true` only if all gates allow the user.
    pub async fn all(&self, user: &U, gates: &[&str]) -> bool {
        for gate in gates {
            if !self.allows(user, gate).await {
                return false;
            }
        }
        true
    }

    /// Authorize a user against a gate, returning an error if denied
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::gates::Gate;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// struct User { role: String }
    ///
    /// let gate: Gate<User> = Gate::new();
    /// gate.define("admin", |user: &User| {
    ///     let role = user.role.clone();
    ///     async move { role == "admin" }
    /// });
    ///
    /// let admin = User { role: "admin".to_string() };
    ///
    /// gate.authorize(&admin, "admin").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize(&self, user: &U, gate: &str) -> AuthorizationResult<()> {
        if self.allows(user, gate).await {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "Gate '{}' denied access",
                gate
            )))
        }
    }

    /// Check if a gate exists
    pub fn has(&self, gate: &str) -> bool {
        let checks = self.checks.lock().unwrap();
        checks.contains_key(gate)
    }

    /// Remove a gate definition
    pub fn forget(&self, gate: &str) -> bool {
        let mut checks = self.checks.lock().unwrap();
        checks.remove(gate).is_some()
    }

    /// Get all defined gate names
    pub fn gates(&self) -> Vec<String> {
        let checks = self.checks.lock().unwrap();
        checks.keys().cloned().collect()
    }
}

impl<U> Default for Gate<U>
where
    U: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<U> Clone for Gate<U> {
    fn clone(&self) -> Self {
        Self {
            checks: Arc::clone(&self.checks),
        }
    }
}

// Global gate instance for convenience
// Note: This uses a generic User type that you'll need to define in your app
// For a typed version, create your own global instance with your User type

/// Global gate instance (type-erased)
///
/// For a production app, you should create your own typed global instance:
///
/// ```rust
/// use rf_auth::authorization::gates::Gate;
/// use once_cell::sync::Lazy;
///
/// struct User { /* your user type */ }
///
/// static GATES: Lazy<Gate<User>> = Lazy::new(|| Gate::new());
///
/// pub fn gate() -> &'static Gate<User> {
///     &GATES
/// }
/// ```
pub fn global_gate<U>() -> Gate<U>
where
    U: Send + Sync + 'static,
{
    Gate::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestUser {
        id: i64,
        role: String,
        permissions: Vec<String>,
    }

    impl TestUser {
        fn has_permission(&self, permission: &str) -> bool {
            self.permissions.iter().any(|p| p == permission)
        }
    }

    #[tokio::test]
    async fn test_define_and_check_gate() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });

        let admin = TestUser {
            id: 1,
            role: "admin".to_string(),
            permissions: vec![],
        };

        let user = TestUser {
            id: 2,
            role: "user".to_string(),
            permissions: vec![],
        };

        assert!(gate.allows(&admin, "admin").await);
        assert!(!gate.allows(&user, "admin").await);
        assert!(gate.denies(&user, "admin").await);
    }

    #[tokio::test]
    async fn test_permission_based_gate() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("edit-posts", |user| {
            let permissions = user.permissions.clone();
            async move { permissions.iter().any(|p| p == "edit_posts") }
        });

        let editor = TestUser {
            id: 1,
            role: "user".to_string(),
            permissions: vec!["edit_posts".to_string()],
        };

        let viewer = TestUser {
            id: 2,
            role: "user".to_string(),
            permissions: vec!["view_posts".to_string()],
        };

        assert!(gate.allows(&editor, "edit-posts").await);
        assert!(!gate.allows(&viewer, "edit-posts").await);
    }

    #[tokio::test]
    async fn test_authorize() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });

        let admin = TestUser {
            id: 1,
            role: "admin".to_string(),
            permissions: vec![],
        };

        let user = TestUser {
            id: 2,
            role: "user".to_string(),
            permissions: vec![],
        };

        assert!(gate.authorize(&admin, "admin").await.is_ok());
        assert!(gate.authorize(&user, "admin").await.is_err());
    }

    #[tokio::test]
    async fn test_any_gate() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });
        gate.define("editor", |user| {
            let role = user.role.clone();
            async move { role == "editor" }
        });

        let admin = TestUser {
            id: 1,
            role: "admin".to_string(),
            permissions: vec![],
        };

        let editor = TestUser {
            id: 2,
            role: "editor".to_string(),
            permissions: vec![],
        };

        let user = TestUser {
            id: 3,
            role: "user".to_string(),
            permissions: vec![],
        };

        assert!(gate.any(&admin, &["admin", "editor"]).await);
        assert!(gate.any(&editor, &["admin", "editor"]).await);
        assert!(!gate.any(&user, &["admin", "editor"]).await);
    }

    #[tokio::test]
    async fn test_all_gates() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("verified", |user| {
            let permissions = user.permissions.clone();
            async move { permissions.iter().any(|p| p == "verified") }
        });
        gate.define("active", |user| {
            let permissions = user.permissions.clone();
            async move { permissions.iter().any(|p| p == "active") }
        });

        let verified_active = TestUser {
            id: 1,
            role: "user".to_string(),
            permissions: vec!["verified".to_string(), "active".to_string()],
        };

        let only_verified = TestUser {
            id: 2,
            role: "user".to_string(),
            permissions: vec!["verified".to_string()],
        };

        assert!(gate.all(&verified_active, &["verified", "active"]).await);
        assert!(!gate.all(&only_verified, &["verified", "active"]).await);
    }

    #[tokio::test]
    async fn test_has_and_forget() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });

        assert!(gate.has("admin"));
        assert!(!gate.has("superadmin"));

        assert!(gate.forget("admin"));
        assert!(!gate.has("admin"));
        assert!(!gate.forget("admin"));
    }

    #[tokio::test]
    async fn test_gates_list() {
        let gate: Gate<TestUser> = Gate::new();

        gate.define("admin", |user| {
            let role = user.role.clone();
            async move { role == "admin" }
        });
        gate.define("editor", |user| {
            let role = user.role.clone();
            async move { role == "editor" }
        });

        let gates = gate.gates();
        assert_eq!(gates.len(), 2);
        assert!(gates.contains(&"admin".to_string()));
        assert!(gates.contains(&"editor".to_string()));
    }

    #[tokio::test]
    async fn test_nonexistent_gate_returns_false() {
        let gate: Gate<TestUser> = Gate::new();

        let user = TestUser {
            id: 1,
            role: "admin".to_string(),
            permissions: vec![],
        };

        assert!(!gate.allows(&user, "nonexistent").await);
    }
}
