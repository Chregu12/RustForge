use crate::error::{AuthorizationError, AuthorizationResult};
use once_cell::sync::Lazy;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A policy for authorizing actions on a specific model type
///
/// # Example
///
/// ```rust,ignore
/// use rf_authorization::Policy;
///
/// struct PostPolicy;
///
/// impl Policy<User, Post> for PostPolicy {
///     fn view(&self, user: Option<&User>, post: &Post) -> bool {
///         post.published || user.map(|u| u.id == post.user_id).unwrap_or(false)
///     }
///
///     fn create(&self, user: &User) -> bool {
///         user.is_verified()
///     }
///
///     fn update(&self, user: &User, post: &Post) -> bool {
///         user.id == post.user_id || user.is_admin()
///     }
///
///     fn delete(&self, user: &User, post: &Post) -> bool {
///         user.id == post.user_id || user.is_admin()
///     }
/// }
/// ```
pub trait Policy<U, M>: Send + Sync {
    /// Check if user can view any instance of this model
    fn view_any(&self, _user: Option<&U>) -> bool {
        true
    }

    /// Check if user can view this specific model instance
    fn view(&self, _user: Option<&U>, _model: &M) -> bool {
        true
    }

    /// Check if user can create new instances
    fn create(&self, _user: &U) -> bool {
        true
    }

    /// Check if user can update this model instance
    fn update(&self, _user: &U, _model: &M) -> bool {
        true
    }

    /// Check if user can delete this model instance
    fn delete(&self, _user: &U, _model: &M) -> bool {
        true
    }

    /// Check if user can restore this soft-deleted model
    fn restore(&self, _user: &U, _model: &M) -> bool {
        true
    }

    /// Check if user can force delete this model (bypass soft delete)
    fn force_delete(&self, _user: &U, _model: &M) -> bool {
        false
    }
}

/// Internal policy storage
type PolicyBox = Box<dyn Any + Send + Sync>;

static POLICY_REGISTRY: Lazy<Arc<RwLock<HashMap<TypeId, PolicyBox>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Policy service for managing and checking policies
pub struct PolicyService;

impl PolicyService {
    /// Register a policy for a model type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::PolicyService;
    ///
    /// PolicyService::register::<Post, PostPolicy>(PostPolicy);
    /// ```
    pub fn register<M: 'static, P: Policy<U, M> + 'static, U: 'static>(policy: P) {
        let type_id = TypeId::of::<M>();
        let boxed = Box::new(Arc::new(policy) as Arc<dyn Policy<U, M>>);

        let mut registry = POLICY_REGISTRY.write().unwrap();
        registry.insert(type_id, boxed);

        tracing::debug!("Registered policy for model type: {:?}", type_id);
    }

    /// Check if a user is authorized for an action on a model
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rf_authorization::PolicyService;
    ///
    /// # fn example(user: User, post: Post) -> Result<(), Box<dyn std::error::Error>> {
    /// let authorized = PolicyService::check("update", Some(&user), &post)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn check<U: 'static, M: 'static>(
        ability: &str,
        user: Option<&U>,
        model: &M,
    ) -> AuthorizationResult<bool> {
        let type_id = TypeId::of::<M>();

        let registry = POLICY_REGISTRY.read().unwrap();

        let policy_box = registry
            .get(&type_id)
            .ok_or_else(|| AuthorizationError::PolicyNotFound(format!("{:?}", type_id)))?;

        // Downcast to the specific policy type
        let policy = policy_box
            .downcast_ref::<Arc<dyn Policy<U, M>>>()
            .ok_or_else(|| AuthorizationError::PolicyNotFound("Type mismatch".to_string()))?;

        let result = match ability {
            "viewAny" => policy.view_any(user),
            "view" => policy.view(user, model),
            "update" => user
                .map(|u| policy.update(u, model))
                .ok_or_else(|| AuthorizationError::Unauthorized("No user".to_string()))?,
            "delete" => user
                .map(|u| policy.delete(u, model))
                .ok_or_else(|| AuthorizationError::Unauthorized("No user".to_string()))?,
            "restore" => user
                .map(|u| policy.restore(u, model))
                .ok_or_else(|| AuthorizationError::Unauthorized("No user".to_string()))?,
            "forceDelete" => user
                .map(|u| policy.force_delete(u, model))
                .ok_or_else(|| AuthorizationError::Unauthorized("No user".to_string()))?,
            _ => return Err(AuthorizationError::InvalidAbility(ability.to_string())),
        };

        Ok(result)
    }

    /// Authorize or throw an error
    pub fn authorize<U: 'static, M: 'static>(
        ability: &str,
        user: Option<&U>,
        model: &M,
    ) -> AuthorizationResult<()> {
        if Self::check(ability, user, model)? {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "Action '{}' is not authorized",
                ability
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestUser {
        id: i32,
        is_admin: bool,
    }

    #[derive(Debug)]
    struct TestPost {
        id: i32,
        user_id: i32,
    }

    struct TestPostPolicy;

    impl Policy<TestUser, TestPost> for TestPostPolicy {
        fn view(&self, _user: Option<&TestUser>, _post: &TestPost) -> bool {
            true
        }

        fn update(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id || user.is_admin
        }

        fn delete(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id || user.is_admin
        }
    }

    #[test]
    fn test_policy_registration() {
        PolicyService::register::<TestPost, TestPostPolicy, TestUser>(TestPostPolicy);
        // If we get here without panicking, registration worked
    }

    #[test]
    fn test_policy_check() {
        PolicyService::register::<TestPost, TestPostPolicy, TestUser>(TestPostPolicy);

        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let post = TestPost { id: 1, user_id: 1 };

        let can_update = PolicyService::check("update", Some(&user), &post).unwrap();
        assert!(can_update);

        let other_post = TestPost { id: 2, user_id: 2 };
        let cannot_update = PolicyService::check("update", Some(&user), &other_post).unwrap();
        assert!(!cannot_update);
    }
}
