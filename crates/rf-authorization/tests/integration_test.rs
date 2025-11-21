//! Integration tests combining Gates, Policies, Middleware, and Permissions

use rf_authorization::{
    gates::Gate,
    middleware::{AuthorizeGateMiddleware, Middleware, Request},
    permissions::{HasPermissions, Permission, Role, UserPermissions},
    policies::{Policy, PolicyRegistry},
};
use std::sync::Arc;

#[derive(Clone)]
struct User {
    id: i64,
    is_admin: bool,
    permissions: UserPermissions,
}

impl User {
    fn new_with_roles(id: i64, roles: Vec<Role>) -> Self {
        Self {
            id,
            is_admin: false,
            permissions: UserPermissions::from_roles(roles),
        }
    }

    fn new_admin(id: i64) -> Self {
        let admin_role = Role::new(1, "admin").with_permissions(vec![
            Permission::new(1, "posts.create"),
            Permission::new(2, "posts.update"),
            Permission::new(3, "posts.delete"),
            Permission::new(4, "users.manage"),
        ]);

        Self {
            id,
            is_admin: true,
            permissions: UserPermissions::from_roles(vec![admin_role]),
        }
    }
}

impl HasPermissions for User {
    fn get_permissions(&self) -> &UserPermissions {
        &self.permissions
    }
}

struct Post {
    id: i64,
    author_id: i64,
    published: bool,
}

struct PostPolicy;

impl Policy<Post> for PostPolicy {
    type User = User;

    fn view(&self, user: Option<&User>, post: &Post) -> bool {
        post.published || user.map(|u| u.id == post.author_id).unwrap_or(false)
    }

    fn create(&self, user: &User) -> bool {
        user.has_permission("posts.create")
    }

    fn update(&self, user: &User, post: &Post) -> bool {
        (user.id == post.author_id && user.has_permission("posts.update")) || user.is_admin
    }

    fn delete(&self, user: &User, _post: &Post) -> bool {
        user.has_permission("posts.delete")
    }
}

#[test]
fn test_integration_admin_can_do_everything() {
    let admin = User::new_admin(1);

    // Test permissions
    assert!(admin.has_permission("posts.create"));
    assert!(admin.has_permission("posts.update"));
    assert!(admin.has_permission("posts.delete"));
    assert!(admin.has_permission("users.manage"));

    // Test policy
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let post = Post {
        id: 1,
        author_id: 2,
        published: true,
    };

    assert!(registry.can(&admin, "create", None::<&Post>));
    assert!(registry.can(&admin, "update", Some(&post)));
    assert!(registry.can(&admin, "delete", Some(&post)));

    // Test gate
    let mut gate = Gate::new();
    gate.define("manage-users", Arc::new(|user: &User, _| {
        user.has_permission("users.manage")
    }));

    assert!(gate.allows(&admin, "manage-users"));
}

#[test]
fn test_integration_editor_with_limited_permissions() {
    let editor_role = Role::new(2, "editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.update"),
    ]);

    let editor = User::new_with_roles(2, vec![editor_role]);

    // Editor can create and update, but not delete
    assert!(editor.has_permission("posts.create"));
    assert!(editor.has_permission("posts.update"));
    assert!(!editor.has_permission("posts.delete"));
    assert!(!editor.has_permission("users.manage"));

    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let own_post = Post {
        id: 1,
        author_id: 2,
        published: true,
    };

    assert!(registry.can(&editor, "create", None::<&Post>));
    assert!(registry.can(&editor, "update", Some(&own_post)));
    assert!(!registry.can(&editor, "delete", Some(&own_post)));
}

#[test]
fn test_integration_viewer_read_only() {
    let viewer_role = Role::new(3, "viewer").with_permissions(vec![]);

    let viewer = User::new_with_roles(3, vec![viewer_role]);

    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let published_post = Post {
        id: 1,
        author_id: 1,
        published: true,
    };

    // Can view published posts
    assert!(registry.can(&viewer, "view", Some(&published_post)));

    // Cannot create, update, or delete
    assert!(!registry.can(&viewer, "create", None::<&Post>));
    assert!(!registry.can(&viewer, "update", Some(&published_post)));
    assert!(!registry.can(&viewer, "delete", Some(&published_post)));
}

#[test]
fn test_integration_multiple_roles() {
    let writer_role = Role::new(4, "writer").with_permissions(vec![
        Permission::new(1, "posts.create"),
    ]);

    let editor_role = Role::new(2, "editor").with_permissions(vec![
        Permission::new(2, "posts.update"),
    ]);

    let user = User::new_with_roles(4, vec![writer_role, editor_role]);

    // User has permissions from both roles
    assert!(user.has_permission("posts.create"));
    assert!(user.has_permission("posts.update"));
    assert!(!user.has_permission("posts.delete"));

    assert!(user.has_role("writer"));
    assert!(user.has_role("editor"));
    assert!(!user.has_role("admin"));
}

#[tokio::test]
async fn test_integration_middleware_with_permissions() {
    let admin = User::new_admin(1);

    let mut gate = Gate::new();
    gate.define("admin-panel", Arc::new(|user: &User, _| {
        user.has_permission("users.manage")
    }));

    let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin-panel");

    let request = Request::new().with_user(admin);
    let result = middleware.handle(request).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_integration_middleware_denies_insufficient_permissions() {
    let editor_role = Role::new(2, "editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
    ]);

    let editor = User::new_with_roles(2, vec![editor_role]);

    let mut gate = Gate::new();
    gate.define("admin-panel", Arc::new(|user: &User, _| {
        user.has_permission("users.manage")
    }));

    let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin-panel");

    let request = Request::new().with_user(editor);
    let result = middleware.handle(request).await;

    assert!(result.is_err());
}

#[test]
fn test_integration_gate_with_permission_check() {
    let admin = User::new_admin(1);
    let editor_role = Role::new(2, "editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.update"),
    ]);
    let editor = User::new_with_roles(2, vec![editor_role]);

    let mut gate = Gate::new();

    // Define gates that check database permissions
    gate.define("create-post", Arc::new(|user: &User, _| {
        user.has_permission("posts.create")
    }));

    gate.define("delete-post", Arc::new(|user: &User, _| {
        user.has_permission("posts.delete")
    }));

    // Admin can do everything
    assert!(gate.allows(&admin, "create-post"));
    assert!(gate.allows(&admin, "delete-post"));

    // Editor has limited permissions
    assert!(gate.allows(&editor, "create-post"));
    assert!(!gate.allows(&editor, "delete-post"));
}

#[test]
fn test_integration_policy_with_permission_check() {
    let admin = User::new_admin(1);
    let editor_role = Role::new(2, "editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.update"),
    ]);
    let editor = User::new_with_roles(2, vec![editor_role]);

    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let editors_post = Post {
        id: 1,
        author_id: 2,
        published: true,
    };

    // Editor can update their own post (has permission and is owner)
    assert!(registry.can(&editor, "update", Some(&editors_post)));

    // Editor cannot delete (no permission)
    assert!(!registry.can(&editor, "delete", Some(&editors_post)));

    // Admin can do everything
    assert!(registry.can(&admin, "update", Some(&editors_post)));
    assert!(registry.can(&admin, "delete", Some(&editors_post)));
}

#[test]
fn test_integration_complex_authorization_scenario() {
    // Create users with different roles
    let admin = User::new_admin(1);

    let senior_editor_role = Role::new(5, "senior-editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.update"),
        Permission::new(3, "posts.delete"),
    ]);
    let senior_editor = User::new_with_roles(2, vec![senior_editor_role]);

    let junior_editor_role = Role::new(6, "junior-editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.update"),
    ]);
    let junior_editor = User::new_with_roles(3, vec![junior_editor_role]);

    let viewer_role = Role::new(3, "viewer").with_permissions(vec![]);
    let viewer = User::new_with_roles(4, vec![viewer_role]);

    // Setup gate
    let mut gate = Gate::new();
    gate.define("delete-any-post", Arc::new(|user: &User, _| {
        user.has_permission("posts.delete")
    }));

    // Setup policy
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let post = Post {
        id: 1,
        author_id: 3,
        published: true,
    };

    // Admin can delete any post
    assert!(gate.allows(&admin, "delete-any-post"));
    assert!(registry.can(&admin, "delete", Some(&post)));

    // Senior editor can delete posts
    assert!(gate.allows(&senior_editor, "delete-any-post"));
    assert!(registry.can(&senior_editor, "delete", Some(&post)));

    // Junior editor cannot delete (no permission)
    assert!(!gate.allows(&junior_editor, "delete-any-post"));
    assert!(!registry.can(&junior_editor, "delete", Some(&post)));

    // Junior editor can update their own post
    assert!(registry.can(&junior_editor, "update", Some(&post)));

    // Viewer cannot do anything except view
    assert!(!gate.allows(&viewer, "delete-any-post"));
    assert!(!registry.can(&viewer, "delete", Some(&post)));
    assert!(!registry.can(&viewer, "update", Some(&post)));
    assert!(registry.can(&viewer, "view", Some(&post))); // Can view published
}

#[test]
fn test_integration_permission_inheritance() {
    // User with multiple roles should inherit all permissions
    let role1 = Role::new(1, "role1").with_permissions(vec![
        Permission::new(1, "perm1"),
        Permission::new(2, "perm2"),
    ]);

    let role2 = Role::new(2, "role2").with_permissions(vec![
        Permission::new(2, "perm2"), // Duplicate - should be deduplicated
        Permission::new(3, "perm3"),
    ]);

    let role3 = Role::new(3, "role3").with_permissions(vec![
        Permission::new(4, "perm4"),
    ]);

    let user = User::new_with_roles(1, vec![role1, role2, role3]);

    assert!(user.has_permission("perm1"));
    assert!(user.has_permission("perm2"));
    assert!(user.has_permission("perm3"));
    assert!(user.has_permission("perm4"));

    let all_perms = user.permissions.get_all_permissions();
    assert_eq!(all_perms.len(), 4); // Should be deduplicated
}
