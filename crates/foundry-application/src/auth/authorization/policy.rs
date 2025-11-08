//! Policy-based Authorization
//!
//! Policies provide resource-specific authorization logic. They are organized
//! around a particular model or resource type and define what actions can be
//! performed on that resource.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{AuthorizationError, AuthorizationResult};

/// Policy callback function type
pub type PolicyCallback = Arc<dyn Fn(&dyn Any, &dyn Any) -> bool + Send + Sync>;

/// Resource policy trait
///
/// Implement this trait to define authorization logic for a specific resource type.
///
/// # Example
///
/// ```rust,ignore
/// struct PostPolicy;
///
/// impl ResourcePolicy<User, Post> for PostPolicy {
///     fn view(&self, user: &User, post: &Post) -> bool {
///         post.is_published || user.id == post.author_id
///     }
///
///     fn update(&self, user: &User, post: &Post) -> bool {
///         user.id == post.author_id
///     }
///
///     fn delete(&self, user: &User, post: &Post) -> bool {
///         user.id == post.author_id || user.is_admin()
///     }
/// }
/// ```
pub trait ResourcePolicy<U, R>: Send + Sync {
    /// Check if user can view the resource
    fn view(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Check if user can create this type of resource
    fn create(&self, _user: &U) -> bool {
        false
    }

    /// Check if user can update the resource
    fn update(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Check if user can delete the resource
    fn delete(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Custom action check
    fn can(&self, _action: &str, _user: &U, _resource: &R) -> bool {
        false
    }
}

/// Policy registry for managing resource policies
pub struct PolicyRegistry {
    // Map: (TypeId of Resource) -> Map: (Action name) -> Callback
    policies: RwLock<HashMap<TypeId, HashMap<String, PolicyCallback>>>,
    before_callbacks: RwLock<Vec<Arc<dyn Fn(&dyn Any, &str, &dyn Any) -> Option<bool> + Send + Sync>>>,
}

impl PolicyRegistry {
    /// Create a new policy registry
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(HashMap::new()),
            before_callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Register a policy for a resource type
    pub async fn register<U, R, P>(&self, policy: P)
    where
        U: 'static,
        R: 'static,
        P: ResourcePolicy<U, R> + 'static,
    {
        let type_id = TypeId::of::<R>();
        let policy = Arc::new(policy);

        let mut policies = self.policies.write().await;
        let resource_policies = policies.entry(type_id).or_insert_with(HashMap::new);

        // Register standard CRUD actions
        let policy_view = policy.clone();
        resource_policies.insert(
            "view".to_string(),
            Arc::new(move |user: &dyn Any, resource: &dyn Any| {
                let user = user.downcast_ref::<U>().unwrap();
                let resource = resource.downcast_ref::<R>().unwrap();
                policy_view.view(user, resource)
            }),
        );

        let policy_update = policy.clone();
        resource_policies.insert(
            "update".to_string(),
            Arc::new(move |user: &dyn Any, resource: &dyn Any| {
                let user = user.downcast_ref::<U>().unwrap();
                let resource = resource.downcast_ref::<R>().unwrap();
                policy_update.update(user, resource)
            }),
        );

        let policy_delete = policy.clone();
        resource_policies.insert(
            "delete".to_string(),
            Arc::new(move |user: &dyn Any, resource: &dyn Any| {
                let user = user.downcast_ref::<U>().unwrap();
                let resource = resource.downcast_ref::<R>().unwrap();
                policy_delete.delete(user, resource)
            }),
        );
    }

    /// Add a before callback for all policy checks
    pub async fn before<F>(&self, callback: F)
    where
        F: Fn(&dyn Any, &str, &dyn Any) -> Option<bool> + Send + Sync + 'static,
    {
        self.before_callbacks.write().await.push(Arc::new(callback));
    }

    /// Check if a user can perform an action on a resource
    pub async fn allows<R>(&self, action: &str, user: &dyn Any, resource: &R) -> bool
    where
        R: 'static,
    {
        // Run before callbacks
        let before_callbacks = self.before_callbacks.read().await;
        for callback in before_callbacks.iter() {
            if let Some(result) = callback(user, action, resource) {
                return result;
            }
        }
        drop(before_callbacks);

        // Check the policy
        let type_id = TypeId::of::<R>();
        let policies = self.policies.read().await;

        policies
            .get(&type_id)
            .and_then(|resource_policies| resource_policies.get(action))
            .map(|callback| callback(user, resource))
            .unwrap_or(false)
    }

    /// Check if a user is denied from performing an action on a resource
    pub async fn denies<R>(&self, action: &str, user: &dyn Any, resource: &R) -> bool
    where
        R: 'static,
    {
        !self.allows(action, user, resource).await
    }

    /// Authorize an action or return an error
    pub async fn authorize<R>(&self, action: &str, user: &dyn Any, resource: &R) -> AuthorizationResult
    where
        R: 'static,
    {
        if self.allows(action, user, resource).await {
            Ok(())
        } else {
            Err(AuthorizationError::AccessDenied)
        }
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global policy registry
static POLICY: once_cell::sync::Lazy<PolicyRegistry> =
    once_cell::sync::Lazy::new(PolicyRegistry::new);

/// Policy - Static interface for policy-based authorization
pub struct Policy;

impl Policy {
    /// Get the global policy registry
    pub fn registry() -> &'static PolicyRegistry {
        &POLICY
    }

    /// Register a policy
    pub async fn register<U, R, P>(policy: P)
    where
        U: 'static,
        R: 'static,
        P: ResourcePolicy<U, R> + 'static,
    {
        Self::registry().register(policy).await;
    }

    /// Add a before callback
    pub async fn before<F>(callback: F)
    where
        F: Fn(&dyn Any, &str, &dyn Any) -> Option<bool> + Send + Sync + 'static,
    {
        Self::registry().before(callback).await;
    }

    /// Check if a user can perform an action on a resource
    pub async fn allows<R>(action: &str, user: &dyn Any, resource: &R) -> bool
    where
        R: 'static,
    {
        Self::registry().allows(action, user, resource).await
    }

    /// Check if a user is denied from performing an action
    pub async fn denies<R>(action: &str, user: &dyn Any, resource: &R) -> bool
    where
        R: 'static,
    {
        Self::registry().denies(action, user, resource).await
    }

    /// Authorize an action or return an error
    pub async fn authorize<R>(action: &str, user: &dyn Any, resource: &R) -> AuthorizationResult
    where
        R: 'static,
    {
        Self::registry().authorize(action, user, resource).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestUser {
        id: i64,
        is_admin: bool,
    }

    #[derive(Debug, Clone)]
    struct TestPost {
        id: i64,
        author_id: i64,
        is_published: bool,
    }

    struct PostPolicy;

    impl ResourcePolicy<TestUser, TestPost> for PostPolicy {
        fn view(&self, user: &TestUser, post: &TestPost) -> bool {
            post.is_published || user.id == post.author_id || user.is_admin
        }

        fn update(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.author_id || user.is_admin
        }

        fn delete(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.author_id || user.is_admin
        }
    }

    #[tokio::test]
    async fn test_policy_view() {
        let registry = PolicyRegistry::new();
        registry.register(PostPolicy).await;

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

        let published_post = TestPost {
            id: 1,
            author_id: 2,
            is_published: true,
        };

        let draft_post = TestPost {
            id: 2,
            author_id: 2,
            is_published: false,
        };

        // Everyone can view published posts
        assert!(registry.allows("view", &admin, &published_post).await);
        assert!(registry.allows("view", &author, &published_post).await);
        assert!(registry.allows("view", &other, &published_post).await);

        // Only author and admin can view drafts
        assert!(registry.allows("view", &admin, &draft_post).await);
        assert!(registry.allows("view", &author, &draft_post).await);
        assert!(registry.denies("view", &other, &draft_post).await);
    }

    #[tokio::test]
    async fn test_policy_update() {
        let registry = PolicyRegistry::new();
        registry.register(PostPolicy).await;

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

        let post = TestPost {
            id: 1,
            author_id: 2,
            is_published: true,
        };

        // Admin and author can update
        assert!(registry.allows("update", &admin, &post).await);
        assert!(registry.allows("update", &author, &post).await);

        // Others cannot
        assert!(registry.denies("update", &other, &post).await);
    }

    #[tokio::test]
    async fn test_policy_delete() {
        let registry = PolicyRegistry::new();
        registry.register(PostPolicy).await;

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

        let post = TestPost {
            id: 1,
            author_id: 2,
            is_published: true,
        };

        // Admin and author can delete
        assert!(registry.allows("delete", &admin, &post).await);
        assert!(registry.allows("delete", &author, &post).await);

        // Others cannot
        assert!(registry.denies("delete", &other, &post).await);
    }

    #[tokio::test]
    async fn test_policy_before_callback() {
        let registry = PolicyRegistry::new();
        registry.register(PostPolicy).await;

        // Before callback that allows all actions for admins
        registry
            .before(|user, _action, _resource| {
                if let Some(user) = user.downcast_ref::<TestUser>() {
                    if user.is_admin {
                        return Some(true);
                    }
                }
                None
            })
            .await;

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let other = TestUser {
            id: 3,
            is_admin: false,
        };

        let post = TestPost {
            id: 1,
            author_id: 2,
            is_published: false,
        };

        // Admin can do anything (via before callback)
        assert!(registry.allows("view", &admin, &post).await);
        assert!(registry.allows("update", &admin, &post).await);
        assert!(registry.allows("delete", &admin, &post).await);

        // Other user is denied (post is not published and they're not the author)
        assert!(registry.denies("view", &other, &post).await);
    }

    #[tokio::test]
    async fn test_authorize() {
        let registry = PolicyRegistry::new();
        registry.register(PostPolicy).await;

        let author = TestUser {
            id: 2,
            is_admin: false,
        };
        let other = TestUser {
            id: 3,
            is_admin: false,
        };

        let post = TestPost {
            id: 1,
            author_id: 2,
            is_published: true,
        };

        // Author can update
        assert!(registry.authorize("update", &author, &post).await.is_ok());

        // Other cannot update
        assert!(registry.authorize("update", &other, &post).await.is_err());
    }
}
