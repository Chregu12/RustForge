//! Authorizable trait for adding authorization methods to user types

use super::error::{AuthorizationError, AuthorizationResult};
use super::registry::global_registry;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for types that can perform authorization checks
///
/// This trait adds authorization methods to your user type, making it easy
/// to check permissions in your application code.
///
/// # Example
///
/// ```rust,ignore
/// use rf_auth::authorization::authorizable::Authorizable;
/// use rf_auth::authorization::policies::post_policy::{User, Post};
///
/// impl Authorizable for User {}
///
/// # async fn example(user: &User, post: &Post) {
/// if user.can("update", post).await {
///     // User can update the post
/// }
///
/// if user.cannot("delete", post).await {
///     // User cannot delete the post
/// }
///
/// // Or use authorize to get a Result
/// let _: Result<(), _> = user.authorize("update", post).await;
/// # }
/// ```
#[async_trait]
pub trait Authorizable: Sized + Send + Sync + 'static {
    /// Check if the user can perform an action on a resource
    ///
    /// # Arguments
    ///
    /// - `action`: The action to check (e.g., "view", "update", "delete")
    /// - `resource`: The resource to check authorization on
    ///
    /// # Returns
    ///
    /// `true` if authorized, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::authorizable::Authorizable;
    ///
    /// # async fn example<U: Authorizable, R>(user: &U, post: &R)
    /// # where R: Send + Sync + 'static {
    /// if user.can("update", post).await {
    ///     // Update the post
    /// }
    /// # }
    /// ```
    async fn can<R>(&self, action: &str, resource: &R) -> bool
    where
        R: Send + Sync + 'static,
    {
        // Must clone Arc before await to avoid holding lock across await point
        use std::any::TypeId;

        let policy_opt = {
            let registry = global_registry().lock().unwrap();
            let type_id = TypeId::of::<R>();
            registry.policies.get(&type_id).map(|arc| Arc::clone(arc))
        };

        if let Some(policy_arc_any) = policy_opt {
            if let Some(policy_arc) = policy_arc_any
                .downcast_ref::<std::sync::Arc<dyn super::policies::Policy<Self, R>>>()
            {
                // Check before hook
                if let Some(result) = policy_arc.before(self, resource).await {
                    return result;
                }

                // Check specific action
                return match action {
                    "view" => policy_arc.view(self, resource).await,
                    "update" => policy_arc.update(self, resource).await,
                    "delete" => policy_arc.delete(self, resource).await,
                    "restore" => policy_arc.restore(self, resource).await,
                    "force_delete" => policy_arc.force_delete(self, resource).await,
                    _ => false,
                };
            }
        }

        false
    }

    /// Check if the user cannot perform an action on a resource
    ///
    /// This is the inverse of `can()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::authorizable::Authorizable;
    ///
    /// # async fn example<U: Authorizable, R>(user: &U, post: &R)
    /// # where R: Send + Sync + 'static {
    /// if user.cannot("delete", post).await {
    ///     return; // User cannot delete
    /// }
    /// # }
    /// ```
    async fn cannot<R>(&self, action: &str, resource: &R) -> bool
    where
        R: Send + Sync + 'static,
    {
        !self.can(action, resource).await
    }

    /// Authorize a user to perform an action on a resource
    ///
    /// Returns `Ok(())` if authorized, or an `AuthorizationError` if not.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::authorizable::Authorizable;
    ///
    /// # async fn example<U: Authorizable, R>(user: &U, post: &R) -> Result<(), Box<dyn std::error::Error>>
    /// # where R: Send + Sync + 'static {
    /// user.authorize("update", post).await?;
    /// // User is authorized, proceed with update
    /// # Ok(())
    /// # }
    /// ```
    async fn authorize<R>(&self, action: &str, resource: &R) -> AuthorizationResult<()>
    where
        R: Send + Sync + 'static,
    {
        if self.can(action, resource).await {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(format!(
                "Action '{}' denied",
                action
            )))
        }
    }

    /// Check if the user can create a resource type
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_auth::authorization::authorizable::Authorizable;
    ///
    /// struct Post;
    ///
    /// # async fn example<U: Authorizable>(user: &U) {
    /// if user.can_create::<Post>().await {
    ///     // User can create posts
    /// }
    /// # }
    /// ```
    async fn can_create<R>(&self) -> bool
    where
        R: Send + Sync + 'static,
    {
        use std::any::TypeId;

        let policy_opt = {
            let registry = global_registry().lock().unwrap();
            let type_id = TypeId::of::<R>();
            registry.policies.get(&type_id).map(|arc| Arc::clone(arc))
        };

        if let Some(policy_arc_any) = policy_opt {
            if let Some(policy_arc) =
                policy_arc_any.downcast_ref::<Arc<dyn super::policies::Policy<Self, R>>>()
            {
                return policy_arc.create(self).await;
            }
        }

        false
    }

    /// Check if the user cannot create a resource type
    ///
    /// This is the inverse of `can_create()`.
    async fn cannot_create<R>(&self) -> bool
    where
        R: Send + Sync + 'static,
    {
        !self.can_create::<R>().await
    }

    /// Authorize a user to create a resource type
    ///
    /// Returns `Ok(())` if authorized, or an `AuthorizationError` if not.
    async fn authorize_create<R>(&self) -> AuthorizationResult<()>
    where
        R: Send + Sync + 'static,
    {
        if self.can_create::<R>().await {
            Ok(())
        } else {
            Err(AuthorizationError::Forbidden(
                "Create action denied".to_string(),
            ))
        }
    }
}

// Blanket implementation for types that are Send + Sync + 'static
// In your app, you can implement Authorizable for your specific User type
// or rely on this blanket implementation

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorization::policies::post_policy::{Post, PostPolicy, User};
    use crate::authorization::registry::global_registry;

    // Register policies for tests
    fn setup() {
        let mut registry = global_registry().lock().unwrap();
        registry.clear();
        registry.register::<User, Post, _>(PostPolicy);
    }

    impl Authorizable for User {}

    #[tokio::test]
    async fn test_can_method() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let own_post = Post {
            id: 1,
            title: "My Post".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        let other_post = Post {
            id: 2,
            title: "Other Post".to_string(),
            content: "Content".to_string(),
            user_id: 2,
            published: true,
        };

        assert!(user.can("update", &own_post).await);
        assert!(!user.can("update", &other_post).await);
    }

    #[tokio::test]
    async fn test_cannot_method() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let other_post = Post {
            id: 2,
            title: "Other Post".to_string(),
            content: "Content".to_string(),
            user_id: 2,
            published: true,
        };

        assert!(user.cannot("update", &other_post).await);
        assert!(!user.cannot("view", &other_post).await);
    }

    #[tokio::test]
    async fn test_authorize_success() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let own_post = Post {
            id: 1,
            title: "My Post".to_string(),
            content: "Content".to_string(),
            user_id: 1,
            published: true,
        };

        assert!(user.authorize("update", &own_post).await.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_failure() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let other_post = Post {
            id: 2,
            title: "Other Post".to_string(),
            content: "Content".to_string(),
            user_id: 2,
            published: true,
        };

        let result = user.authorize("update", &other_post).await;
        assert!(result.is_err());

        match result {
            Err(AuthorizationError::Forbidden(_)) => (),
            _ => panic!("Expected Forbidden error"),
        }
    }

    #[tokio::test]
    async fn test_can_create() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        let guest = User {
            id: 2,
            email: "guest@example.com".to_string(),
            roles: vec!["guest".to_string()],
        };

        assert!(user.can_create::<Post>().await);
        assert!(!guest.can_create::<Post>().await);
    }

    #[tokio::test]
    async fn test_cannot_create() {
        setup();

        let guest = User {
            id: 2,
            email: "guest@example.com".to_string(),
            roles: vec!["guest".to_string()],
        };

        assert!(guest.cannot_create::<Post>().await);
    }

    #[tokio::test]
    async fn test_authorize_create_success() {
        setup();

        let user = User {
            id: 1,
            email: "user@example.com".to_string(),
            roles: vec!["user".to_string()],
        };

        assert!(user.authorize_create::<Post>().await.is_ok());
    }

    #[tokio::test]
    async fn test_authorize_create_failure() {
        setup();

        let guest = User {
            id: 2,
            email: "guest@example.com".to_string(),
            roles: vec!["guest".to_string()],
        };

        assert!(guest.authorize_create::<Post>().await.is_err());
    }
}
