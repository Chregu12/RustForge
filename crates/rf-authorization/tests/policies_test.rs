//! Comprehensive tests for Policies

use rf_authorization::policies::{Policy, PolicyRegistry};

#[derive(Clone)]
struct User {
    id: i64,
    is_admin: bool,
    is_verified: bool,
}

impl User {
    fn new_admin(id: i64) -> Self {
        Self {
            id,
            is_admin: true,
            is_verified: true,
        }
    }

    fn new_regular(id: i64) -> Self {
        Self {
            id,
            is_admin: false,
            is_verified: true,
        }
    }

    fn new_unverified(id: i64) -> Self {
        Self {
            id,
            is_admin: false,
            is_verified: false,
        }
    }
}

struct Post {
    id: i64,
    author_id: i64,
    published: bool,
}

impl Post {
    fn new(id: i64, author_id: i64, published: bool) -> Self {
        Self {
            id,
            author_id,
            published,
        }
    }
}

struct PostPolicy;

impl Policy<Post> for PostPolicy {
    type User = User;

    fn view(&self, user: Option<&User>, post: &Post) -> bool {
        post.published || user.map(|u| u.id == post.author_id).unwrap_or(false)
    }

    fn create(&self, user: &User) -> bool {
        user.is_verified
    }

    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id || user.is_admin
    }

    fn delete(&self, user: &User, _post: &Post) -> bool {
        user.is_admin
    }

    fn restore(&self, user: &User, _post: &Post) -> bool {
        user.is_admin
    }

    fn force_delete(&self, user: &User, _post: &Post) -> bool {
        user.is_admin
    }
}

#[test]
fn test_policy_registration() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    assert!(registry.has::<Post>());
}

#[test]
fn test_policy_view_published() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::new_regular(1);
    let published_post = Post::new(1, 2, true);

    assert!(registry.can(&user, "view", Some(&published_post)));
}

#[test]
fn test_policy_view_own_unpublished() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::new_regular(1);
    let own_post = Post::new(1, 1, false);

    assert!(registry.can(&user, "view", Some(&own_post)));
}

#[test]
fn test_policy_view_others_unpublished() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::new_regular(1);
    let others_post = Post::new(1, 2, false);

    assert!(!registry.can(&user, "view", Some(&others_post)));
}

#[test]
fn test_policy_create_verified() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let verified_user = User::new_regular(1);

    assert!(registry.can::<Post, User>(&verified_user, "create", None));
}

#[test]
fn test_policy_create_unverified() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let unverified_user = User::new_unverified(1);

    assert!(!registry.can::<Post, User>(&unverified_user, "create", None));
}

#[test]
fn test_policy_update_owner() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let owner = User::new_regular(1);
    let post = Post::new(1, 1, true);

    assert!(registry.authorize(&owner, "update", Some(&post)).is_ok());
}

#[test]
fn test_policy_update_non_owner() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::new_regular(1);
    let post = Post::new(1, 2, true);

    assert!(registry.authorize(&user, "update", Some(&post)).is_err());
}

#[test]
fn test_policy_update_admin() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let post = Post::new(1, 2, true);

    assert!(registry.authorize(&admin, "update", Some(&post)).is_ok());
}

#[test]
fn test_policy_delete_admin_only() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let regular = User::new_regular(2);
    let post = Post::new(1, 2, true);

    assert!(registry.authorize(&admin, "delete", Some(&post)).is_ok());
    assert!(registry.authorize(&regular, "delete", Some(&post)).is_err());
}

#[test]
fn test_policy_restore_admin_only() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let regular = User::new_regular(2);
    let post = Post::new(1, 2, true);

    assert!(registry.can(&admin, "restore", Some(&post)));
    assert!(!registry.can(&regular, "restore", Some(&post)));
}

#[test]
fn test_policy_force_delete_admin_only() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let regular = User::new_regular(2);
    let post = Post::new(1, 2, true);

    assert!(registry.can(&admin, "forceDelete", Some(&post)));
    assert!(!registry.can(&regular, "forceDelete", Some(&post)));
}

#[test]
fn test_policy_not_found() {
    let registry = PolicyRegistry::new();

    let user = User::new_regular(1);
    let post = Post::new(1, 1, true);

    let result = registry.authorize(&user, "update", Some(&post));
    assert!(result.is_err());
}

#[test]
fn test_policy_invalid_action() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::new_regular(1);
    let post = Post::new(1, 1, true);

    let result = registry.check(&user, "invalid-action", Some(&post));
    assert!(result.is_err());
}

#[test]
fn test_policy_can_cannot() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let regular = User::new_regular(2);
    let post = Post::new(1, 2, true);

    assert!(registry.can(&admin, "delete", Some(&post)));
    assert!(registry.cannot(&regular, "delete", Some(&post)));
}

#[test]
fn test_policy_forget() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    assert!(registry.has::<Post>());

    registry.forget::<Post>();

    assert!(!registry.has::<Post>());
}

#[test]
fn test_policy_clone() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let cloned = registry.clone();

    assert!(cloned.has::<Post>());

    let user = User::new_regular(1);
    let post = Post::new(1, 1, true);

    assert!(cloned.can(&user, "update", Some(&post)));
}

#[test]
fn test_multiple_policies() {
    struct Comment {
        id: i64,
        author_id: i64,
    }

    struct CommentPolicy;

    impl Policy<Comment> for CommentPolicy {
        type User = User;

        fn delete(&self, user: &User, comment: &Comment) -> bool {
            user.id == comment.author_id || user.is_admin
        }
    }

    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);
    registry.register::<Comment, CommentPolicy>(CommentPolicy);

    assert!(registry.has::<Post>());
    assert!(registry.has::<Comment>());

    let user = User::new_regular(1);
    let post = Post::new(1, 1, true);
    let comment = Comment {
        id: 1,
        author_id: 1,
    };

    assert!(registry.can(&user, "update", Some(&post)));
    assert!(registry.can(&user, "delete", Some(&comment)));
}

#[test]
fn test_policy_with_multiple_users() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let owner = User::new_regular(2);
    let other = User::new_regular(3);

    let post = Post::new(1, 2, true);

    // Admin and owner can update
    assert!(registry.can(&admin, "update", Some(&post)));
    assert!(registry.can(&owner, "update", Some(&post)));
    assert!(!registry.can(&other, "update", Some(&post)));

    // Only admin can delete
    assert!(registry.can(&admin, "delete", Some(&post)));
    assert!(!registry.can(&owner, "delete", Some(&post)));
    assert!(!registry.can(&other, "delete", Some(&post)));
}
