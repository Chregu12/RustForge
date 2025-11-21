//! Enhanced Policies - Model-Based Authorization
//!
//! Policies provide a clean way to organize authorization logic around models.
//! Each policy corresponds to a model and defines abilities like view, create, update, delete.

use crate::error::{AuthorizationError, AuthorizationResult};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Trait for defining policies on models
///
/// Implement this trait to define authorization rules for a model.
///
/// # Example
///
/// ```rust
/// use rf_authorization::policies::Policy;
///
/// #[derive(Clone)]
/// struct User {
///     id: i64,
///     is_admin: bool,
/// }
///
/// struct Post {
///     id: i64,
///     author_id: i64,
///     published: bool,
/// }
///
/// struct PostPolicy;
///
/// impl Policy<Post> for PostPolicy {
///     type User = User;
///
///     fn view(&self, user: Option<&User>, post: &Post) -> bool {
///         // Anyone can view published posts
///         // Only author can view unpublished posts
///         post.published || user.map(|u| u.id == post.author_id).unwrap_or(false)
///     }
///
///     fn create(&self, user: &User) -> bool {
///         // Any authenticated user can create posts
///         true
///     }
///
///     fn update(&self, user: &User, post: &Post) -> bool {
///         // Only author or admin can update
///         user.id == post.author_id || user.is_admin
///     }
///
///     fn delete(&self, user: &User, post: &Post) -> bool {
///         // Only admin can delete
///         user.is_admin
///     }
/// }
/// ```
pub trait Policy<T>: Send + Sync {
    /// The user type for authorization
    type User;

    /// Check if user can view any instances of this model
    fn view_any(&self, _user: Option<&Self::User>) -> bool {
        true
    }

    /// Check if user can view this specific model instance
    fn view(&self, _user: Option<&Self::User>, _model: &T) -> bool {
        true
    }

    /// Check if user can create new instances
    fn create(&self, _user: &Self::User) -> bool {
        false
    }

    /// Check if user can update this model instance
    fn update(&self, _user: &Self::User, _model: &T) -> bool {
        false
    }

    /// Check if user can delete this model instance
    fn delete(&self, _user: &Self::User, _model: &T) -> bool {
        false
    }

    /// Check if user can restore this soft-deleted model
    fn restore(&self, _user: &Self::User, _model: &T) -> bool {
        false
    }

    /// Check if user can force delete this model (bypass soft delete)
    fn force_delete(&self, _user: &Self::User, _model: &T) -> bool {
        false
    }
}

/// Registry for managing policies
///
/// This allows you to register policies for models and then check authorization.
///
/// # Example
///
/// ```rust
/// use rf_authorization::policies::{Policy, PolicyRegistry};
///
/// # #[derive(Clone)]
/// # struct User { id: i64, is_admin: bool }
/// # struct Post { id: i64, author_id: i64 }
/// # struct PostPolicy;
/// # impl Policy<Post> for PostPolicy {
/// #     type User = User;
/// #     fn update(&self, user: &User, post: &Post) -> bool {
/// #         user.id == post.author_id
/// #     }
/// # }
/// let mut registry = PolicyRegistry::new();
///
/// // Register policy
/// registry.register::<Post, PostPolicy>(PostPolicy);
///
/// let user = User { id: 1, is_admin: false };
/// let post = Post { id: 1, author_id: 1 };
///
/// // Check authorization
/// assert!(registry.authorize(&user, "update", Some(&post)).is_ok());
/// ```
pub struct PolicyRegistry {
    policies: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl PolicyRegistry {
    /// Create a new policy registry
    pub fn new() -> Self {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a policy for a model type
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::policies::{Policy, PolicyRegistry};
    /// # #[derive(Clone)]
    /// # struct User { id: i64 }
    /// # struct Post { id: i64 }
    /// # struct PostPolicy;
    /// # impl Policy<Post> for PostPolicy {
    /// #     type User = User;
    /// # }
    /// let mut registry = PolicyRegistry::new();
    /// registry.register::<Post, PostPolicy>(PostPolicy);
    /// ```
    pub fn register<T: 'static, P>(&mut self, policy: P)
    where
        P: Policy<T> + 'static,
        P::User: 'static,
    {
        let type_id = TypeId::of::<T>();
        let boxed: Box<dyn Any + Send + Sync> = Box::new(Arc::new(policy) as Arc<dyn Policy<T, User = P::User>>);

        let mut policies = self.policies.lock().unwrap();
        policies.insert(type_id, boxed);

        tracing::debug!("Registered policy for type: {:?}", type_id);
    }

    /// Check if a policy is registered for a type
    pub fn has<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let policies = self.policies.lock().unwrap();
        policies.contains_key(&type_id)
    }

    /// Remove a policy
    pub fn forget<T: 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        let mut policies = self.policies.lock().unwrap();
        policies.remove(&type_id);
    }

    /// Authorize an action or return an error
    ///
    /// # Arguments
    ///
    /// * `user` - The user attempting the action
    /// * `action` - The action name (view, create, update, delete, etc.)
    /// * `model` - Optional model instance (None for "create" action)
    ///
    /// # Errors
    ///
    /// Returns `AuthorizationError::PolicyNotFound` if no policy is registered.
    /// Returns `AuthorizationError::Forbidden` if the action is not allowed.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_authorization::policies::{Policy, PolicyRegistry};
    /// # #[derive(Clone)]
    /// # struct User { id: i64 }
    /// # struct Post { id: i64, author_id: i64 }
    /// # struct PostPolicy;
    /// # impl Policy<Post> for PostPolicy {
    /// #     type User = User;
    /// #     fn update(&self, user: &User, post: &Post) -> bool {
    /// #         user.id == post.author_id
    /// #     }
    /// # }
    /// # let mut registry = PolicyRegistry::new();
    /// # registry.register::<Post, PostPolicy>(PostPolicy);
    /// # let user = User { id: 1 };
    /// # let post = Post { id: 1, author_id: 1 };
    /// registry.authorize(&user, "update", Some(&post))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn authorize<T, U>(
        &self,
        user: &U,
        action: &str,
        model: Option<&T>,
    ) -> AuthorizationResult<()>
    where
        T: 'static,
        U: 'static,
    {
        if self.check(user, action, model)? {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "User not authorized to {} this resource",
                action
            )))
        }
    }

    /// Check if a user can perform an action
    ///
    /// Returns `true` if allowed, `false` if denied.
    ///
    /// # Errors
    ///
    /// Returns `AuthorizationError::PolicyNotFound` if no policy is registered.
    pub fn check<T, U>(
        &self,
        user: &U,
        action: &str,
        model: Option<&T>,
    ) -> AuthorizationResult<bool>
    where
        T: 'static,
        U: 'static,
    {
        let type_id = TypeId::of::<T>();

        let policies = self.policies.lock().unwrap();

        let policy_box = policies
            .get(&type_id)
            .ok_or_else(|| AuthorizationError::PolicyNotFound(format!("No policy for type: {:?}", type_id)))?;

        // Downcast to the specific policy type
        let policy = policy_box
            .downcast_ref::<Arc<dyn Policy<T, User = U>>>()
            .ok_or_else(|| AuthorizationError::PolicyNotFound("Type mismatch".to_string()))?;

        let allowed = match action {
            "viewAny" => policy.view_any(Some(user)),
            "view" => model.map(|m| policy.view(Some(user), m)).unwrap_or(false),
            "create" => policy.create(user),
            "update" => model.map(|m| policy.update(user, m)).unwrap_or(false),
            "delete" => model.map(|m| policy.delete(user, m)).unwrap_or(false),
            "restore" => model.map(|m| policy.restore(user, m)).unwrap_or(false),
            "forceDelete" => model.map(|m| policy.force_delete(user, m)).unwrap_or(false),
            _ => {
                return Err(AuthorizationError::InvalidAbility(format!(
                    "Unknown action: {}",
                    action
                )))
            }
        };

        Ok(allowed)
    }

    /// Check if a user can perform an action (returns bool, doesn't throw)
    ///
    /// Returns `false` if policy not found or action denied.
    pub fn can<T, U>(&self, user: &U, action: &str, model: Option<&T>) -> bool
    where
        T: 'static,
        U: 'static,
    {
        self.check(user, action, model).unwrap_or(false)
    }

    /// Check if a user cannot perform an action
    pub fn cannot<T, U>(&self, user: &U, action: &str, model: Option<&T>) -> bool
    where
        T: 'static,
        U: 'static,
    {
        !self.can(user, action, model)
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PolicyRegistry {
    fn clone(&self) -> Self {
        Self {
            policies: Arc::clone(&self.policies),
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
    }

    struct TestPost {
        id: i64,
        author_id: i64,
        published: bool,
    }

    struct TestPostPolicy;

    impl Policy<TestPost> for TestPostPolicy {
        type User = TestUser;

        fn view(&self, user: Option<&TestUser>, post: &TestPost) -> bool {
            post.published || user.map(|u| u.id == post.author_id).unwrap_or(false)
        }

        fn create(&self, _user: &TestUser) -> bool {
            true
        }

        fn update(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.author_id || user.is_admin
        }

        fn delete(&self, user: &TestUser, _post: &TestPost) -> bool {
            user.is_admin
        }
    }

    #[test]
    fn test_policy_registration() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        assert!(registry.has::<TestPost>());
    }

    #[test]
    fn test_policy_update() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let owner = TestUser {
            id: 1,
            is_admin: false,
        };
        let other = TestUser {
            id: 2,
            is_admin: false,
        };
        let post = TestPost {
            id: 1,
            author_id: 1,
            published: true,
        };

        assert!(registry.authorize(&owner, "update", Some(&post)).is_ok());
        assert!(registry.authorize(&other, "update", Some(&post)).is_err());
    }

    #[test]
    fn test_policy_delete_admin_only() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let regular = TestUser {
            id: 2,
            is_admin: false,
        };
        let post = TestPost {
            id: 1,
            author_id: 2,
            published: true,
        };

        assert!(registry.authorize(&admin, "delete", Some(&post)).is_ok());
        assert!(registry.authorize(&regular, "delete", Some(&post)).is_err());
    }

    #[test]
    fn test_policy_create() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let user = TestUser {
            id: 1,
            is_admin: false,
        };

        assert!(registry.authorize::<TestPost, TestUser>(&user, "create", None).is_ok());
    }

    #[test]
    fn test_policy_view_published() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let published_post = TestPost {
            id: 1,
            author_id: 2,
            published: true,
        };
        let unpublished_post = TestPost {
            id: 2,
            author_id: 2,
            published: false,
        };

        assert!(registry.can(&user, "view", Some(&published_post)));
        assert!(!registry.can(&user, "view", Some(&unpublished_post)));
    }

    #[test]
    fn test_policy_view_own_unpublished() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let own_unpublished = TestPost {
            id: 1,
            author_id: 1,
            published: false,
        };

        assert!(registry.can(&user, "view", Some(&own_unpublished)));
    }

    #[test]
    fn test_policy_not_found() {
        let registry = PolicyRegistry::new();

        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let post = TestPost {
            id: 1,
            author_id: 1,
            published: true,
        };

        let result = registry.authorize(&user, "update", Some(&post));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthorizationError::PolicyNotFound(_)));
    }

    #[test]
    fn test_policy_can_cannot() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let regular = TestUser {
            id: 2,
            is_admin: false,
        };
        let post = TestPost {
            id: 1,
            author_id: 2,
            published: true,
        };

        assert!(registry.can(&admin, "delete", Some(&post)));
        assert!(registry.cannot(&regular, "delete", Some(&post)));
    }

    #[test]
    fn test_policy_forget() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        assert!(registry.has::<TestPost>());

        registry.forget::<TestPost>();

        assert!(!registry.has::<TestPost>());
    }

    #[test]
    fn test_policy_clone() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let cloned = registry.clone();
        assert!(cloned.has::<TestPost>());
    }

    #[test]
    fn test_invalid_action() {
        let mut registry = PolicyRegistry::new();
        registry.register::<TestPost, TestPostPolicy>(TestPostPolicy);

        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let post = TestPost {
            id: 1,
            author_id: 1,
            published: true,
        };

        let result = registry.check(&user, "invalid-action", Some(&post));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthorizationError::InvalidAbility(_)));
    }
}
