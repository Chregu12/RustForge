//! Example Post policy implementation

use super::Policy;
use async_trait::async_trait;

/// Example User struct for demonstration
///
/// In a real application, this would come from your domain model
#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub roles: Vec<String>,
}

impl User {
    /// Check if user has admin role
    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r == "admin")
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, _permission: &str) -> bool {
        // In a real implementation, this would check against a permissions table
        // For now, admins have all permissions
        self.is_admin()
    }
}

/// Example Post struct for demonstration
///
/// In a real application, this would come from your domain model
#[derive(Debug, Clone)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub user_id: i64,
    pub published: bool,
}

/// Policy for Post resources
///
/// This policy implements authorization rules for blog posts:
/// - Admins can do anything
/// - Anyone can view published posts
/// - Only the post author can update their posts
/// - Only the post author can delete their posts
///
/// # Example
///
/// ```rust
/// use rf_auth::authorization::policies::{Policy, post_policy::{User, Post, PostPolicy}};
///
/// # async fn example() {
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
/// let policy = PostPolicy;
///
/// assert!(policy.view(&user, &post).await);
/// assert!(policy.update(&user, &post).await);
/// # }
/// ```
pub struct PostPolicy;

#[async_trait]
impl Policy<User, Post> for PostPolicy {
    async fn before(&self, user: &User, _resource: &Post) -> Option<bool> {
        // Admins can do anything
        if user.is_admin() {
            return Some(true);
        }
        None
    }

    async fn view(&self, _user: &User, post: &Post) -> bool {
        // Anyone can view published posts
        // Authors can view their own drafts (handled in before or this check)
        post.published
    }

    async fn create(&self, user: &User) -> bool {
        // Any authenticated user can create posts
        // In a real app, you might check for specific permissions
        user.has_role("user") || user.has_role("author")
    }

    async fn update(&self, user: &User, post: &Post) -> bool {
        // Only the post author can update
        user.id == post.user_id
    }

    async fn delete(&self, user: &User, post: &Post) -> bool {
        // Only the post author can delete
        user.id == post.user_id
    }

    async fn restore(&self, user: &User, post: &Post) -> bool {
        // Only the post author can restore
        user.id == post.user_id
    }

    async fn force_delete(&self, user: &User, _post: &Post) -> bool {
        // Only admins can force delete (but this is already handled in before)
        user.is_admin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorization::policies::PolicyCheck;

    fn create_user(id: i64, roles: Vec<&str>) -> User {
        User {
            id,
            email: format!("user{}@example.com", id),
            roles: roles.iter().map(|r| r.to_string()).collect(),
        }
    }

    fn create_post(id: i64, user_id: i64, published: bool) -> Post {
        Post {
            id,
            title: format!("Post {}", id),
            content: "Content".to_string(),
            user_id,
            published,
        }
    }

    #[tokio::test]
    async fn test_admin_can_do_everything() {
        let admin = create_user(1, vec!["admin"]);
        let post = create_post(1, 2, false);
        let policy = PostPolicy;

        assert!(policy.check(&admin, "view", &post).await);
        assert!(policy.check(&admin, "update", &post).await);
        assert!(policy.check(&admin, "delete", &post).await);
        assert!(policy.check(&admin, "restore", &post).await);
        assert!(policy.check(&admin, "force_delete", &post).await);
    }

    #[tokio::test]
    async fn test_user_can_view_published_posts() {
        let user = create_user(1, vec!["user"]);
        let published_post = create_post(1, 2, true);
        let draft_post = create_post(2, 2, false);
        let policy = PostPolicy;

        assert!(policy.view(&user, &published_post).await);
        assert!(!policy.view(&user, &draft_post).await);
    }

    #[tokio::test]
    async fn test_user_can_create_posts() {
        let user = create_user(1, vec!["user"]);
        let author = create_user(2, vec!["author"]);
        let guest = create_user(3, vec!["guest"]);
        let policy = PostPolicy;

        assert!(policy.create(&user).await);
        assert!(policy.create(&author).await);
        assert!(!policy.create(&guest).await);
    }

    #[tokio::test]
    async fn test_user_can_only_update_own_posts() {
        let user = create_user(1, vec!["user"]);
        let own_post = create_post(1, 1, true);
        let other_post = create_post(2, 2, true);
        let policy = PostPolicy;

        assert!(policy.update(&user, &own_post).await);
        assert!(!policy.update(&user, &other_post).await);
    }

    #[tokio::test]
    async fn test_user_can_only_delete_own_posts() {
        let user = create_user(1, vec!["user"]);
        let own_post = create_post(1, 1, true);
        let other_post = create_post(2, 2, true);
        let policy = PostPolicy;

        assert!(policy.delete(&user, &own_post).await);
        assert!(!policy.delete(&user, &other_post).await);
    }

    #[tokio::test]
    async fn test_only_admin_can_force_delete() {
        let user = create_user(1, vec!["user"]);
        let admin = create_user(2, vec!["admin"]);
        let post = create_post(1, 1, true);
        let policy = PostPolicy;

        assert!(!policy.force_delete(&user, &post).await);
        assert!(policy.force_delete(&admin, &post).await);
    }
}
