//! Integration tests for rf-authorization gates and policies

use rf_authorization::{
    gates::Gate,
    permissions::{Permission, Role, UserPermissions},
    policies::{Policy, PolicyRegistry},
    AuthorizationError,
};
use std::sync::Arc;

// ── Shared test types ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct User {
    id: i64,
    is_admin: bool,
    permissions: Vec<String>,
}

impl User {
    fn regular(id: i64) -> Self {
        Self { id, is_admin: false, permissions: vec![] }
    }
    fn admin(id: i64) -> Self {
        Self { id, is_admin: true, permissions: vec![] }
    }
    fn with_permission(mut self, p: &str) -> Self {
        self.permissions.push(p.into());
        self
    }
    fn has_perm(&self, p: &str) -> bool {
        self.permissions.contains(&p.to_string())
    }
}

struct Post {
    id: i64,
    author_id: i64,
    published: bool,
}

struct Comment {
    id: i64,
    author_id: i64,
}

// ── Gates ─────────────────────────────────────────────────────────────────────

#[test]
fn gate_allows_returns_true_for_authorized_user() {
    let mut gate = Gate::new();
    gate.define("publish-post", Arc::new(|user: &User, _| user.is_admin));

    let admin = User::admin(1);
    assert!(gate.allows(&admin, "publish-post"));
}

#[test]
fn gate_allows_returns_false_for_unauthorized_user() {
    let mut gate = Gate::new();
    gate.define("publish-post", Arc::new(|user: &User, _| user.is_admin));

    let regular = User::regular(2);
    assert!(!gate.allows(&regular, "publish-post"));
}

#[test]
fn gate_denies_is_inverse_of_allows() {
    let mut gate = Gate::new();
    gate.define("admin-only", Arc::new(|user: &User, _| user.is_admin));

    let admin = User::admin(1);
    let regular = User::regular(2);
    assert!(!gate.denies(&admin, "admin-only"));
    assert!(gate.denies(&regular, "admin-only"));
}

#[test]
fn gate_authorize_ok_for_allowed_user() {
    let mut gate = Gate::new();
    gate.define("edit-profile", Arc::new(|_: &User, _| true));
    assert!(gate.authorize(&User::regular(1), "edit-profile").is_ok());
}

#[test]
fn gate_authorize_err_for_denied_user() {
    let mut gate = Gate::new();
    gate.define("delete-user", Arc::new(|user: &User, _| user.is_admin));
    let result = gate.authorize(&User::regular(1), "delete-user");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AuthorizationError::Forbidden(_)));
}

#[test]
fn gate_default_deny_for_undefined_ability() {
    let gate: Gate<User> = Gate::new();
    let admin = User::admin(1);
    assert!(!gate.allows(&admin, "undefined-ability"));
    assert!(gate.denies(&admin, "undefined-ability"));
}

#[test]
fn gate_has_returns_true_after_define() {
    let mut gate = Gate::new();
    gate.define("test-ability", Arc::new(|_: &User, _| true));
    assert!(gate.has("test-ability"));
    assert!(!gate.has("other-ability"));
}

#[test]
fn gate_forget_removes_ability() {
    let mut gate = Gate::new();
    gate.define("temporary", Arc::new(|_: &User, _| true));
    assert!(gate.has("temporary"));
    gate.forget("temporary");
    assert!(!gate.has("temporary"));
}

#[test]
fn gate_allows_all_requires_every_ability() {
    let mut gate = Gate::new();
    gate.define("read", Arc::new(|u: &User, _| u.has_perm("read")));
    gate.define("write", Arc::new(|u: &User, _| u.has_perm("write")));

    let both = User::regular(1).with_permission("read").with_permission("write");
    let one = User::regular(2).with_permission("read");

    assert!(gate.allows_all(&both, &["read", "write"]));
    assert!(!gate.allows_all(&one, &["read", "write"]));
}

#[test]
fn gate_allows_any_requires_at_least_one_ability() {
    let mut gate = Gate::new();
    gate.define("read", Arc::new(|u: &User, _| u.has_perm("read")));
    gate.define("write", Arc::new(|u: &User, _| u.has_perm("write")));

    let one = User::regular(1).with_permission("read");
    let none = User::regular(2);

    assert!(gate.allows_any(&one, &["read", "write"]));
    assert!(!gate.allows_any(&none, &["read", "write"]));
}

#[test]
fn gate_define_many_registers_all_abilities() {
    let mut gate: Gate<User> = Gate::new();
    gate.define("a1", Arc::new(|_: &User, _| true));
    gate.define("a2", Arc::new(|_: &User, _| false));
    gate.define("a3", Arc::new(|u: &User, _| u.is_admin));

    assert!(gate.has("a1"));
    assert!(gate.has("a2"));
    assert!(gate.has("a3"));
}

#[test]
fn gate_clone_shares_abilities() {
    let mut gate = Gate::new();
    gate.define("shared", Arc::new(|_: &User, _| true));
    let clone = gate.clone();
    assert!(clone.has("shared"));
}

#[test]
fn gate_ability_callback_receives_ability_name() {
    let mut gate = Gate::new();
    // Callback uses the ability parameter itself for lookup
    gate.define(
        "dynamic",
        Arc::new(|user: &User, ability: &str| user.has_perm(ability)),
    );

    let user = User::regular(1).with_permission("dynamic");
    assert!(gate.allows(&user, "dynamic"));
    assert!(!gate.allows(&User::regular(2), "dynamic"));
}

#[test]
fn gate_all_lists_all_defined_abilities() {
    let mut gate = Gate::new();
    gate.define("x", Arc::new(|_: &User, _| true));
    gate.define("y", Arc::new(|_: &User, _| true));
    gate.define("z", Arc::new(|_: &User, _| false));

    let names = gate.all();
    assert!(names.contains(&"x".to_string()));
    assert!(names.contains(&"y".to_string()));
    assert!(names.contains(&"z".to_string()));
}

// ── Policies ──────────────────────────────────────────────────────────────────

struct PostPolicy;

impl Policy<Post> for PostPolicy {
    type User = User;

    fn view(&self, user: Option<&User>, post: &Post) -> bool {
        post.published || user.map(|u| u.id == post.author_id).unwrap_or(false)
    }

    fn create(&self, _user: &User) -> bool {
        true
    }

    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id || user.is_admin
    }

    fn delete(&self, user: &User, _post: &Post) -> bool {
        user.is_admin
    }
}

struct CommentPolicy;

impl Policy<Comment> for CommentPolicy {
    type User = User;

    fn update(&self, user: &User, comment: &Comment) -> bool {
        user.id == comment.author_id
    }

    fn delete(&self, user: &User, comment: &Comment) -> bool {
        user.id == comment.author_id || user.is_admin
    }
}

#[test]
fn policy_registered_is_detectable() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);
    assert!(registry.has::<Post>());
    assert!(!registry.has::<Comment>());
}

#[test]
fn policy_owner_can_update() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let owner = User::regular(1);
    let post = Post { id: 1, author_id: 1, published: true };

    assert!(registry.authorize(&owner, "update", Some(&post)).is_ok());
}

#[test]
fn policy_non_owner_cannot_update() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let other = User::regular(2);
    let post = Post { id: 1, author_id: 1, published: true };

    assert!(registry.authorize(&other, "update", Some(&post)).is_err());
}

#[test]
fn policy_admin_can_update_any_post() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::admin(99);
    let post = Post { id: 1, author_id: 42, published: false };

    assert!(registry.authorize(&admin, "update", Some(&post)).is_ok());
}

#[test]
fn policy_only_admin_can_delete() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::admin(1);
    let regular = User::regular(2);
    let post = Post { id: 1, author_id: 2, published: true };

    assert!(registry.authorize(&admin, "delete", Some(&post)).is_ok());
    assert!(registry.authorize(&regular, "delete", Some(&post)).is_err());
}

#[test]
fn policy_any_user_can_create() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::regular(5);
    assert!(registry.authorize::<Post, User>(&user, "create", None).is_ok());
}

#[test]
fn policy_published_post_viewable_by_anyone() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let stranger = User::regular(99);
    let post = Post { id: 1, author_id: 1, published: true };
    assert!(registry.can(&stranger, "view", Some(&post)));
}

#[test]
fn policy_unpublished_post_not_viewable_by_strangers() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let stranger = User::regular(99);
    let post = Post { id: 1, author_id: 1, published: false };
    assert!(!registry.can(&stranger, "view", Some(&post)));
}

#[test]
fn policy_unpublished_post_viewable_by_author() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let author = User::regular(1);
    let post = Post { id: 1, author_id: 1, published: false };
    assert!(registry.can(&author, "view", Some(&post)));
}

#[test]
fn policy_cannot_returns_opposite_of_can() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::admin(1);
    let regular = User::regular(2);
    let post = Post { id: 1, author_id: 2, published: true };

    assert!(!registry.cannot(&admin, "delete", Some(&post)));
    assert!(registry.cannot(&regular, "delete", Some(&post)));
}

#[test]
fn policy_not_found_returns_error() {
    let registry = PolicyRegistry::new();
    let user = User::regular(1);
    let post = Post { id: 1, author_id: 1, published: true };

    let result = registry.authorize(&user, "update", Some(&post));
    assert!(matches!(
        result.unwrap_err(),
        AuthorizationError::PolicyNotFound(_)
    ));
}

#[test]
fn policy_invalid_action_returns_error() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let user = User::regular(1);
    let post = Post { id: 1, author_id: 1, published: true };

    let result = registry.check(&user, "nonexistent-action", Some(&post));
    assert!(matches!(
        result.unwrap_err(),
        AuthorizationError::InvalidAbility(_)
    ));
}

#[test]
fn multiple_policies_are_independent() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);
    registry.register::<Comment, CommentPolicy>(CommentPolicy);

    let user = User::regular(1);
    let post = Post { id: 1, author_id: 1, published: true };
    let comment = Comment { id: 1, author_id: 1 };

    assert!(registry.authorize(&user, "update", Some(&post)).is_ok());
    assert!(registry.authorize(&user, "update", Some(&comment)).is_ok());
}

#[test]
fn policy_forget_removes_registration() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);
    registry.forget::<Post>();
    assert!(!registry.has::<Post>());
}

#[test]
fn policy_registry_clone_shares_state() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);
    let cloned = registry.clone();
    assert!(cloned.has::<Post>());
}

// ── Permissions (RBAC) ────────────────────────────────────────────────────────

#[test]
fn permission_has_returns_true_for_granted_permission() {
    let role = Role::new(1, "editor").with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.edit"),
    ]);
    let perms = UserPermissions::from_roles(vec![role]);
    assert!(perms.has("posts.create"));
    assert!(perms.has("posts.edit"));
    assert!(!perms.has("posts.delete"));
}

#[test]
fn multiple_roles_merge_permissions() {
    let r1 = Role::new(1, "writer").with_permissions(vec![Permission::new(1, "posts.create")]);
    let r2 = Role::new(2, "editor").with_permissions(vec![Permission::new(2, "posts.edit")]);
    let perms = UserPermissions::from_roles(vec![r1, r2]);
    assert!(perms.has("posts.create"));
    assert!(perms.has("posts.edit"));
}

#[test]
fn duplicate_permissions_across_roles_deduplicated() {
    let r1 = Role::new(1, "a").with_permissions(vec![Permission::new(1, "x.do")]);
    let r2 = Role::new(2, "b").with_permissions(vec![Permission::new(1, "x.do")]);
    let perms = UserPermissions::from_roles(vec![r1, r2]);
    assert_eq!(perms.get_all_permissions().len(), 1);
}
