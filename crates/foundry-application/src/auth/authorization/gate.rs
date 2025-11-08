//! Gate-based Authorization
//!
//! Gates provide a simple way to determine if a user is authorized to perform
//! a given action. Unlike policies, gates are not tied to a specific model or resource.

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{AuthorizationError, AuthorizationResult};

/// Gate callback function type
pub type GateCallback = Arc<dyn Fn(&dyn Any) -> bool + Send + Sync>;

/// Before callback (runs before all gate checks)
pub type BeforeCallback = Arc<dyn Fn(&dyn Any) -> Option<bool> + Send + Sync>;

/// After callback (runs after gate check)
pub type AfterCallback = Arc<dyn Fn(&dyn Any, bool) -> bool + Send + Sync>;

/// Gate registry for storing and managing authorization gates
pub struct GateRegistry {
    gates: RwLock<HashMap<String, GateCallback>>,
    before_callbacks: RwLock<Vec<BeforeCallback>>,
    after_callbacks: RwLock<Vec<AfterCallback>>,
}

impl GateRegistry {
    /// Create a new gate registry
    pub fn new() -> Self {
        Self {
            gates: RwLock::new(HashMap::new()),
            before_callbacks: RwLock::new(Vec::new()),
            after_callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Define a new gate
    pub async fn define<F>(&self, name: impl Into<String>, callback: F)
    where
        F: Fn(&dyn Any) -> bool + Send + Sync + 'static,
    {
        self.gates
            .write()
            .await
            .insert(name.into(), Arc::new(callback));
    }

    /// Add a before callback (runs before all gate checks)
    ///
    /// Before callbacks can return:
    /// - Some(true) - Allow the action immediately without checking the gate
    /// - Some(false) - Deny the action immediately without checking the gate
    /// - None - Continue to check the gate normally
    pub async fn before<F>(&self, callback: F)
    where
        F: Fn(&dyn Any) -> Option<bool> + Send + Sync + 'static,
    {
        self.before_callbacks.write().await.push(Arc::new(callback));
    }

    /// Add an after callback (runs after gate check)
    pub async fn after<F>(&self, callback: F)
    where
        F: Fn(&dyn Any, bool) -> bool + Send + Sync + 'static,
    {
        self.after_callbacks.write().await.push(Arc::new(callback));
    }

    /// Check if a gate allows the action
    pub async fn allows(&self, name: &str, args: &dyn Any) -> bool {
        // Run before callbacks
        let before_callbacks = self.before_callbacks.read().await;
        for callback in before_callbacks.iter() {
            if let Some(result) = callback(args) {
                return result;
            }
        }
        drop(before_callbacks);

        // Check the gate
        let gates = self.gates.read().await;
        let result = gates
            .get(name)
            .map(|callback| callback(args))
            .unwrap_or(false);
        drop(gates);

        // Run after callbacks
        let after_callbacks = self.after_callbacks.read().await;
        let final_result =
            after_callbacks
                .iter()
                .fold(result, |acc, callback| callback(args, acc));

        final_result
    }

    /// Check if a gate denies the action
    pub async fn denies(&self, name: &str, args: &dyn Any) -> bool {
        !self.allows(name, args).await
    }

    /// Authorize an action or return an error
    pub async fn authorize(&self, name: &str, args: &dyn Any) -> AuthorizationResult {
        if self.allows(name, args).await {
            Ok(())
        } else {
            Err(AuthorizationError::AccessDenied)
        }
    }

    /// Check if a gate exists
    pub async fn has(&self, name: &str) -> bool {
        self.gates.read().await.contains_key(name)
    }

    /// Remove a gate
    pub async fn remove(&self, name: &str) {
        self.gates.write().await.remove(name);
    }
}

impl Default for GateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global gate instance
static GATE: once_cell::sync::Lazy<GateRegistry> = once_cell::sync::Lazy::new(GateRegistry::new);

/// Gate - Static interface for authorization checks
pub struct Gate;

impl Gate {
    /// Get the global gate registry
    pub fn registry() -> &'static GateRegistry {
        &GATE
    }

    /// Define a new gate
    pub async fn define<F>(name: impl Into<String>, callback: F)
    where
        F: Fn(&dyn Any) -> bool + Send + Sync + 'static,
    {
        Self::registry().define(name, callback).await;
    }

    /// Add a before callback
    pub async fn before<F>(callback: F)
    where
        F: Fn(&dyn Any) -> Option<bool> + Send + Sync + 'static,
    {
        Self::registry().before(callback).await;
    }

    /// Add an after callback
    pub async fn after<F>(callback: F)
    where
        F: Fn(&dyn Any, bool) -> bool + Send + Sync + 'static,
    {
        Self::registry().after(callback).await;
    }

    /// Check if a gate allows the action
    pub async fn allows(name: &str, args: &dyn Any) -> bool {
        Self::registry().allows(name, args).await
    }

    /// Check if a gate denies the action
    pub async fn denies(name: &str, args: &dyn Any) -> bool {
        Self::registry().denies(name, args).await
    }

    /// Authorize an action or return an error
    pub async fn authorize(name: &str, args: &dyn Any) -> AuthorizationResult {
        Self::registry().authorize(name, args).await
    }

    /// Check if a gate exists
    pub async fn has(name: &str) -> bool {
        Self::registry().has(name).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestUser {
        id: i64,
        is_admin: bool,
    }

    #[derive(Debug, Clone, Copy)]
    struct TestPost {
        author_id: i64,
    }

    #[tokio::test]
    async fn test_simple_gate() {
        let registry = GateRegistry::new();

        registry
            .define("view-dashboard", |_args| {
                // Simple gate that always allows
                true
            })
            .await;

        assert!(registry.allows("view-dashboard", &()).await);
        assert!(!registry.denies("view-dashboard", &()).await);
    }

    #[tokio::test]
    async fn test_gate_with_user() {
        let registry = GateRegistry::new();

        registry
            .define("admin-only", |args| {
                let user = args.downcast_ref::<TestUser>().unwrap();
                user.is_admin
            })
            .await;

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let regular_user = TestUser {
            id: 2,
            is_admin: false,
        };

        assert!(registry.allows("admin-only", &admin).await);
        assert!(registry.denies("admin-only", &regular_user).await);
    }

    #[tokio::test]
    async fn test_gate_with_multiple_args() {
        let registry = GateRegistry::new();

        registry
            .define("edit-post", |args| {
                let (user, post) = args.downcast_ref::<(TestUser, TestPost)>().unwrap();
                user.id == post.author_id || user.is_admin
            })
            .await;

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let author = TestUser {
            id: 2,
            is_admin: false,
        };
        let other = TestUser {
            id: 3,
            is_admin: false,
        };
        let post = TestPost { author_id: 2 };

        // Admin can edit any post
        assert!(registry.allows("edit-post", &(admin, post)).await);

        // Author can edit their own post
        assert!(registry.allows("edit-post", &(author, post)).await);

        // Other users cannot edit
        assert!(registry.denies("edit-post", &(other, post)).await);
    }

    #[tokio::test]
    async fn test_before_callback() {
        let registry = GateRegistry::new();

        // Before callback that allows all admins
        registry
            .before(|args| {
                if let Some(user) = args.downcast_ref::<TestUser>() {
                    if user.is_admin {
                        return Some(true); // Allow all actions for admins
                    }
                }
                None // Continue with normal gate check
            })
            .await;

        registry
            .define("restricted", |_args| {
                false // This gate normally denies everyone
            })
            .await;

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let regular_user = TestUser {
            id: 2,
            is_admin: false,
        };

        // Admin bypasses the gate
        assert!(registry.allows("restricted", &admin).await);

        // Regular user is denied by the gate
        assert!(registry.denies("restricted", &regular_user).await);
    }

    #[tokio::test]
    async fn test_authorize() {
        let registry = GateRegistry::new();

        registry
            .define("test-gate", |args| {
                let value = args.downcast_ref::<bool>().unwrap();
                *value
            })
            .await;

        assert!(registry.authorize("test-gate", &true).await.is_ok());
        assert!(registry.authorize("test-gate", &false).await.is_err());
    }

    #[tokio::test]
    async fn test_has_gate() {
        let registry = GateRegistry::new();

        assert!(!registry.has("nonexistent").await);

        registry.define("existing", |_| true).await;

        assert!(registry.has("existing").await);
    }
}
