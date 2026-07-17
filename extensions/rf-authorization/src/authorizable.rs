use crate::error::AuthorizationResult;
use crate::policy::PolicyService;

/// Trait for models that can be authorized
///
/// This trait provides convenient authorization methods on user/model types.
///
/// # Example
///
/// ```rust,ignore
/// use rf_authorization::{Authorizable, Policy, PolicyService};
///
/// struct User {
///     id: i32,
/// }
///
/// impl Authorizable for User {
///     type User = Self;
///
///     fn get_user(&self) -> Option<&Self::User> {
///         Some(self)
///     }
/// }
///
/// struct Post {
///     id: i32,
///     user_id: i32,
/// }
///
/// struct PostPolicy;
///
/// impl Policy<User, Post> for PostPolicy {
///     fn update(&self, user: &User, post: &Post) -> bool {
///         user.id == post.user_id
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Register policy
/// PolicyService::register::<Post, PostPolicy, User>(PostPolicy);
///
/// let user = User { id: 1 };
/// let post = Post { id: 1, user_id: 1 };
///
/// // Use the trait method
/// user.authorize("update", &post)?;
/// # Ok(())
/// # }
/// ```
pub trait Authorizable: Sized {
    /// The user type for authorization
    type User: 'static;

    /// Get the user for authorization checks
    fn get_user(&self) -> Option<&Self::User>;

    /// Check if authorized for an ability on a model
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use rf_authorization::Authorizable;
    /// # fn example(user: impl Authorizable, post: &Post) -> Result<(), Box<dyn std::error::Error>> {
    /// if user.can("update", post)? {
    ///     // User can update post
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn can<M: 'static>(&self, ability: &str, model: &M) -> AuthorizationResult<bool> {
        PolicyService::check(ability, self.get_user(), model)
    }

    /// Check if NOT authorized for an ability on a model
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use rf_authorization::Authorizable;
    /// # fn example(user: impl Authorizable, post: &Post) -> Result<(), Box<dyn std::error::Error>> {
    /// if user.cannot("delete", post)? {
    ///     return Err("Cannot delete this post".into());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn cannot<M: 'static>(&self, ability: &str, model: &M) -> AuthorizationResult<bool> {
        Ok(!self.can(ability, model)?)
    }

    /// Authorize or throw an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use rf_authorization::Authorizable;
    /// # fn example(user: impl Authorizable, post: &Post) -> Result<(), Box<dyn std::error::Error>> {
    /// user.authorize("update", post)?;
    /// // If we get here, user is authorized
    /// # Ok(())
    /// # }
    /// ```
    fn authorize<M: 'static>(&self, ability: &str, model: &M) -> AuthorizationResult<()> {
        PolicyService::authorize(ability, self.get_user(), model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Policy;

    #[derive(Debug)]
    struct TestUser {
        id: i32,
    }

    impl Authorizable for TestUser {
        type User = Self;

        fn get_user(&self) -> Option<&Self::User> {
            Some(self)
        }
    }

    #[derive(Debug)]
    struct TestPost {
        id: i32,
        user_id: i32,
    }

    struct TestPostPolicy;

    impl Policy<TestUser, TestPost> for TestPostPolicy {
        fn update(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id
        }

        fn delete(&self, user: &TestUser, post: &TestPost) -> bool {
            user.id == post.user_id
        }
    }

    #[test]
    fn test_authorizable_can() {
        PolicyService::register::<TestPost, TestPostPolicy, TestUser>(TestPostPolicy);

        let user = TestUser { id: 1 };
        let post = TestPost { id: 1, user_id: 1 };

        assert!(user.can("update", &post).unwrap());
        assert!(!user.can("update", &TestPost { id: 2, user_id: 2 }).unwrap());
    }

    #[test]
    fn test_authorizable_cannot() {
        PolicyService::register::<TestPost, TestPostPolicy, TestUser>(TestPostPolicy);

        let user = TestUser { id: 1 };
        let other_post = TestPost { id: 2, user_id: 2 };

        assert!(user.cannot("delete", &other_post).unwrap());
    }

    #[test]
    fn test_authorizable_authorize() {
        PolicyService::register::<TestPost, TestPostPolicy, TestUser>(TestPostPolicy);

        let user = TestUser { id: 1 };
        let post = TestPost { id: 1, user_id: 1 };
        let other_post = TestPost { id: 2, user_id: 2 };

        assert!(user.authorize("update", &post).is_ok());
        assert!(user.authorize("update", &other_post).is_err());
    }
}
