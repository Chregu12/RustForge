//! Policy registry for managing and looking up policies by type

use super::error::{AuthorizationError, AuthorizationResult};
use super::policies::{Policy, PolicyCheck};
use once_cell::sync::Lazy;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Registry for storing and retrieving policies
///
/// The registry maps resource types to their policies, allowing dynamic
/// lookup of policies at runtime.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::registry::PolicyRegistry;
/// use rf_auth::authorization::policies::{Policy, post_policy::{User, Post, PostPolicy}};
///
/// let mut registry = PolicyRegistry::new();
/// registry.register::<User, Post, _>(PostPolicy);
///
/// # async fn example(registry: &PolicyRegistry, user: &User, post: &Post) {
/// let can_update = registry.check(user, "update", post).await;
/// # }
/// ```
pub struct PolicyRegistry {
    pub(crate) policies: HashMap<TypeId, Arc<dyn std::any::Any + Send + Sync>>,
}

impl PolicyRegistry {
    /// Create a new empty policy registry
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Register a policy for a specific user and resource type
    ///
    /// # Type Parameters
    ///
    /// - `U`: User type
    /// - `R`: Resource type
    /// - `P`: Policy type implementing `Policy<U, R>`
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::registry::PolicyRegistry;
    /// use rf_auth::authorization::policies::post_policy::{User, Post, PostPolicy};
    ///
    /// let mut registry = PolicyRegistry::new();
    /// registry.register::<User, Post, _>(PostPolicy);
    /// ```
    pub fn register<U, R, P>(&mut self, policy: P)
    where
        U: 'static,
        R: 'static,
        P: Policy<U, R> + 'static,
    {
        let type_id = TypeId::of::<R>();
        self.policies
            .insert(type_id, Arc::new(Arc::new(policy) as Arc<dyn Policy<U, R>>));
    }

    /// Check if a user can perform an action on a resource
    ///
    /// # Arguments
    ///
    /// - `user`: The user to check authorization for
    /// - `action`: The action to check (e.g., "view", "update", "delete")
    /// - `resource`: The resource to check authorization on
    ///
    /// # Returns
    ///
    /// `true` if the user is authorized, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::registry::PolicyRegistry;
    /// use rf_auth::authorization::policies::post_policy::{User, Post, PostPolicy};
    ///
    /// # async fn example() {
    /// let mut registry = PolicyRegistry::new();
    /// registry.register::<User, Post, _>(PostPolicy);
    ///
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    ///     roles: vec!["user".to_string()],
    /// };
    ///
    /// let post = Post {
    ///     id: 1,
    ///     title: "My Post".to_string(),
    ///     content: "Content".to_string(),
    ///     user_id: 1,
    ///     published: true,
    /// };
    ///
    /// let can_update = registry.check(&user, "update", &post).await;
    /// assert!(can_update);
    /// # }
    /// ```
    pub async fn check<U, R>(&self, user: &U, action: &str, resource: &R) -> bool
    where
        U: Sync + 'static,
        R: Sync + 'static,
    {
        let type_id = TypeId::of::<R>();

        if let Some(policy_arc_any) = self.policies.get(&type_id) {
            if let Some(policy_arc) = policy_arc_any.downcast_ref::<Arc<dyn Policy<U, R>>>() {
                // Check before hook
                if let Some(result) = policy_arc.before(user, resource).await {
                    return result;
                }

                // Check specific action
                return match action {
                    "view" => policy_arc.view(user, resource).await,
                    "update" => policy_arc.update(user, resource).await,
                    "delete" => policy_arc.delete(user, resource).await,
                    "restore" => policy_arc.restore(user, resource).await,
                    "force_delete" => policy_arc.force_delete(user, resource).await,
                    _ => false,
                };
            }
        }

        false
    }

    /// Check if a user can create a resource (no resource instance needed)
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::registry::PolicyRegistry;
    /// use rf_auth::authorization::policies::post_policy::{User, Post, PostPolicy};
    ///
    /// # async fn example() {
    /// let mut registry = PolicyRegistry::new();
    /// registry.register::<User, Post, _>(PostPolicy);
    ///
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    ///     roles: vec!["user".to_string()],
    /// };
    ///
    /// let can_create = registry.check_create::<User, Post>(&user).await;
    /// # }
    /// ```
    pub async fn check_create<U, R>(&self, user: &U) -> bool
    where
        U: Sync + 'static,
        R: 'static,
    {
        let type_id = TypeId::of::<R>();

        if let Some(policy_arc_any) = self.policies.get(&type_id) {
            if let Some(policy_arc) = policy_arc_any.downcast_ref::<Arc<dyn Policy<U, R>>>() {
                return policy_arc.create(user).await;
            }
        }

        false
    }

    /// Authorize a user to perform an action on a resource, returning an error if denied
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::registry::PolicyRegistry;
    /// use rf_auth::authorization::policies::post_policy::{User, Post, PostPolicy};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut registry = PolicyRegistry::new();
    /// registry.register::<User, Post, _>(PostPolicy);
    ///
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    ///     roles: vec!["user".to_string()],
    /// };
    ///
    /// let post = Post {
    ///     id: 1,
    ///     title: "My Post".to_string(),
    ///     content: "Content".to_string(),
    ///     user_id: 1,
    ///     published: true,
    /// };
    ///
    /// registry.authorize(&user, "update", &post).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn authorize<U, R>(
        &self,
        user: &U,
        action: &str,
        resource: &R,
    ) -> AuthorizationResult<()>
    where
        U: Sync + 'static,
        R: Sync + 'static,
    {
        if self.check(user, action, resource).await {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "Action '{}' denied on resource",
                action
            )))
        }
    }

    /// Check if a policy is registered for a resource type
    pub fn has<R>(&self) -> bool
    where
        R: 'static,
    {
        let type_id = TypeId::of::<R>();
        self.policies.contains_key(&type_id)
    }

    /// Get the number of registered policies
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }

    /// Clear all registered policies
    pub fn clear(&mut self) {
        self.policies.clear();
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global policy registry instance
///
/// This provides a convenient global registry for most applications.
/// For more control, you can create your own registry instances.
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::registry::{global_registry, PolicyRegistry};
/// use rf_auth::authorization::policies::post_policy::{User, Post, PostPolicy};
///
/// // Register a policy globally
/// {
///     let mut registry = global_registry().lock().unwrap();
///     registry.register::<User, Post, _>(PostPolicy);
/// }
///
/// // Use it later
/// # async fn example(user: &User, post: &Post) {
/// let registry = global_registry().lock().unwrap();
/// let can_update = registry.check(user, "update", post).await;
/// # }
/// ```
pub static GLOBAL_REGISTRY: Lazy<Mutex<PolicyRegistry>> =
    Lazy::new(|| Mutex::new(PolicyRegistry::new()));

/// Get a reference to the global policy registry
pub fn global_registry() -> &'static Mutex<PolicyRegistry> {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorization::policies::post_policy::{Post, PostPolicy, User};
    use async_trait::async_trait;

    #[derive(Debug, Clone)]
    struct Comment {
        id: i64,
        post_id: i64,
        user_id: i64,
        content: String,
    }

    struct CommentPolicy;

    #[async_trait]
    impl Policy<User, Comment> for CommentPolicy {
        async fn view(&self, _user: &User, _comment: &Comment) -> bool {
            true
        }

        async fn create(&self, _user: &User) -> bool {
            true
        }

        async fn update(&self, user: &User, comment: &Comment) -> bool {
            user.id == comment.user_id
        }

        async fn delete(&self, user: &User, comment: &Comment) -> bool {
            user.id == comment.user_id
        }
    }

    #[tokio::test]
    async fn test_register_and_check_policy() {
        let mut registry = PolicyRegistry::new();
        registry.register::<User, Post, _>(PostPolicy);

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let post = Post {
            id: 1,
            title: "Test".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        assert!(registry.check(&user, "update", &post).await);
        assert!(registry.check(&user, "delete", &post).await);
    }

    #[tokio::test]
    async fn test_multiple_policies() {
        let mut registry = PolicyRegistry::new();
        registry.register::<User, Post, _>(PostPolicy);
        registry.register::<User, Comment, _>(CommentPolicy);

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let post = Post {
            id: 1,
            title: "Test".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        let comment = Comment {
            id: 1,
            post_id: 1,
            user_id: 1,
            content: "Great post!".to_string(),
        };

        assert!(registry.check(&user, "update", &post).await);
        assert!(registry.check(&user, "update", &comment).await);
    }

    #[tokio::test]
    async fn test_check_create() {
        let mut registry = PolicyRegistry::new();
        registry.register::<User, Post, _>(PostPolicy);

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        assert!(registry.check_create::<User, Post>(&user).await);
    }

    #[tokio::test]
    async fn test_authorize_success() {
        let mut registry = PolicyRegistry::new();
        registry.register::<User, Post, _>(PostPolicy);

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let post = Post {
            id: 1,
            title: "Test".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        assert!(registry.authorize(&user, "update", &post).await.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_failure() {
        let mut registry = PolicyRegistry::new();
        registry.register::<User, Post, _>(PostPolicy);

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let other_post = Post {
            id: 2,
            title: "Test".to_string(),
            content: "Content".to_string(),
            user_id: 2,
            published: true,
        };

        assert!(registry
            .authorize(&user, "update", &other_post)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn test_has_policy() {
        let mut registry = PolicyRegistry::new();

        assert!(!registry.has::<Post>());

        registry.register::<User, Post, _>(PostPolicy);

        assert!(registry.has::<Post>());
    }

    #[tokio::test]
    async fn test_nonexistent_policy_returns_false() {
        let registry = PolicyRegistry::new();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let post = Post {
            id: 1,
            title: "Test".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        assert!(!registry.check(&user, "update", &post).await);
    }

    #[tokio::test]
    async fn test_len_and_clear() {
        let mut registry = PolicyRegistry::new();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        registry.register::<User, Post, _>(PostPolicy);
        registry.register::<User, Comment, _>(CommentPolicy);

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());

        registry.clear();

        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }
}
