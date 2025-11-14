//! Policy trait and implementations for resource-based authorization

use async_trait::async_trait;

pub mod post_policy;

/// Policy trait for resource-based authorization
///
/// Policies determine what actions a user can perform on specific resources.
/// Each method corresponds to a common CRUD operation, but you can add custom
/// methods as needed.
///
/// # Type Parameters
///
/// - `U`: User type
/// - `R`: Resource type
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::policies::Policy;
/// use async_trait::async_trait;
///
/// struct User {
///     id: i64,
///     is_admin: bool,
/// }
///
/// struct Post {
///     id: i64,
///     user_id: i64,
/// }
///
/// struct PostPolicy;
///
/// #[async_trait]
/// impl Policy<User, Post> for PostPolicy {
///     async fn before(&self, user: &User, _resource: &Post) -> Option<bool> {
///         // Admins can do anything
///         if user.is_admin {
///             return Some(true);
///         }
///         None
///     }
///
///     async fn update(&self, user: &User, post: &Post) -> bool {
///         user.id == post.user_id
///     }
/// }
/// ```
#[async_trait]
pub trait Policy<U, R>: Send + Sync {
    /// Called before any other policy check
    ///
    /// If this returns `Some(true)`, the user is authorized without checking other methods.
    /// If this returns `Some(false)`, the user is denied without checking other methods.
    /// If this returns `None`, the specific action method will be called.
    ///
    /// This is useful for implementing admin overrides or global denials.
    async fn before(&self, _user: &U, _resource: &R) -> Option<bool> {
        None
    }

    /// Determine if the user can view the resource
    async fn view(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Determine if the user can create a new resource
    ///
    /// Note: This method doesn't receive a resource instance since
    /// we're checking if the user can create a new one.
    async fn create(&self, _user: &U) -> bool {
        false
    }

    /// Determine if the user can update the resource
    async fn update(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Determine if the user can delete the resource
    async fn delete(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Determine if the user can restore a soft-deleted resource
    async fn restore(&self, _user: &U, _resource: &R) -> bool {
        false
    }

    /// Determine if the user can force delete a resource
    async fn force_delete(&self, _user: &U, _resource: &R) -> bool {
        false
    }
}

/// Helper trait for policy checking with the `before` hook
#[async_trait]
pub trait PolicyCheck<U, R>: Policy<U, R>
where
    U: Sync,
    R: Sync,
{
    /// Check if a user can perform an action on a resource
    ///
    /// This method handles the `before` hook automatically.
    async fn check(&self, user: &U, action: &str, resource: &R) -> bool {
        // Check before hook
        if let Some(result) = self.before(user, resource).await {
            return result;
        }

        // Check specific action
        match action {
            "view" => self.view(user, resource).await,
            "update" => self.update(user, resource).await,
            "delete" => self.delete(user, resource).await,
            "restore" => self.restore(user, resource).await,
            "force_delete" => self.force_delete(user, resource).await,
            _ => false,
        }
    }

    /// Check if a user can create a resource (no resource instance needed)
    async fn check_create(&self, user: &U) -> bool {
        self.create(user).await
    }
}

// Blanket implementation
impl<U, R, T> PolicyCheck<U, R> for T
where
    T: Policy<U, R>,
    U: Sync,
    R: Sync,
{}

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
        user_id: i64,
        published: bool,
    }

    struct TestPostPolicy;

    #[async_trait]
    impl Policy<TestUser, TestPost> for TestPostPolicy {
        async fn before(&self, user: &TestUser, _resource: &TestPost) -> Option<bool> {
            if user.is_admin {
                return Some(true);
            }
            None
        }

        async fn view(&self, _user: &TestUser, post: &TestPost) -> bool {
            post.published
        }

        async fn update(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id
        }

        async fn delete(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id
        }
    }

    #[tokio::test]
    async fn test_admin_can_do_anything() {
        let admin = TestUser {
            id: 1,
            is_admin: true,
        };
        let post = TestPost {
            id: 1,
            user_id: 2,
            published: false,
        };
        let policy = TestPostPolicy;

        assert!(policy.check(&admin, "view", &post).await);
        assert!(policy.check(&admin, "update", &post).await);
        assert!(policy.check(&admin, "delete", &post).await);
    }

    #[tokio::test]
    async fn test_user_can_only_see_published_posts() {
        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let published_post = TestPost {
            id: 1,
            user_id: 2,
            published: true,
        };
        let draft_post = TestPost {
            id: 2,
            user_id: 2,
            published: false,
        };
        let policy = TestPostPolicy;

        assert!(policy.check(&user, "view", &published_post).await);
        assert!(!policy.check(&user, "view", &draft_post).await);
    }

    #[tokio::test]
    async fn test_user_can_only_update_own_posts() {
        let user = TestUser {
            id: 1,
            is_admin: false,
        };
        let own_post = TestPost {
            id: 1,
            user_id: 1,
            published: true,
        };
        let other_post = TestPost {
            id: 2,
            user_id: 2,
            published: true,
        };
        let policy = TestPostPolicy;

        assert!(policy.check(&user, "update", &own_post).await);
        assert!(!policy.check(&user, "update", &other_post).await);
    }
}
