//! Basic usage examples for rf-authorization

use rf_authorization::{
    gates::Gate,
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

    fn new_editor(id: i64) -> Self {
        let editor_role = Role::new(2, "editor").with_permissions(vec![
            Permission::new(1, "posts.create"),
            Permission::new(2, "posts.update"),
        ]);

        Self {
            id,
            is_admin: false,
            permissions: UserPermissions::from_roles(vec![editor_role]),
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

fn main() {
    println!("=== rf-authorization Examples ===\n");

    // Example 1: Gates - Simple Permission Checks
    println!("1. Gates - Simple Permission Checks");
    gates_example();
    println!();

    // Example 2: Policies - Model-Based Authorization
    println!("2. Policies - Model-Based Authorization");
    policies_example();
    println!();

    // Example 3: Database-Backed Permissions (RBAC)
    println!("3. Database-Backed Permissions (RBAC)");
    permissions_example();
    println!();

    // Example 4: Combined Example
    println!("4. Combined Example - Gates + Policies + Permissions");
    combined_example();
}

fn gates_example() {
    let mut gate = Gate::new();

    // Define gates
    gate.define(
        "create-post",
        Arc::new(|user: &User, _| user.has_permission("posts.create")),
    );

    gate.define(
        "delete-post",
        Arc::new(|user: &User, _| user.has_permission("posts.delete")),
    );

    gate.define(
        "manage-users",
        Arc::new(|user: &User, _| user.has_permission("users.manage")),
    );

    let admin = User::new_admin(1);
    let editor = User::new_editor(2);

    // Check permissions
    println!(
        "  Admin can create post: {}",
        gate.allows(&admin, "create-post")
    );
    println!(
        "  Admin can delete post: {}",
        gate.allows(&admin, "delete-post")
    );
    println!(
        "  Admin can manage users: {}",
        gate.allows(&admin, "manage-users")
    );
    println!();
    println!(
        "  Editor can create post: {}",
        gate.allows(&editor, "create-post")
    );
    println!(
        "  Editor can delete post: {}",
        gate.denies(&editor, "delete-post")
    );
    println!(
        "  Editor can manage users: {}",
        gate.denies(&editor, "manage-users")
    );
}

fn policies_example() {
    let mut registry = PolicyRegistry::new();
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let editor = User::new_editor(2);

    let published_post = Post {
        id: 1,
        author_id: 2,
        published: true,
    };

    let editors_draft = Post {
        id: 2,
        author_id: 2,
        published: false,
    };

    // Check authorization
    println!(
        "  Admin can update editor's published post: {}",
        registry.can(&admin, "update", Some(&published_post))
    );
    println!(
        "  Admin can delete editor's post: {}",
        registry.can(&admin, "delete", Some(&published_post))
    );
    println!();
    println!(
        "  Editor can update own published post: {}",
        registry.can(&editor, "update", Some(&published_post))
    );
    println!(
        "  Editor can update own draft: {}",
        registry.can(&editor, "update", Some(&editors_draft))
    );
    println!(
        "  Editor can delete own post: {}",
        registry.cannot(&editor, "delete", Some(&published_post))
    );
}

fn permissions_example() {
    let admin = User::new_admin(1);
    let editor = User::new_editor(2);

    println!("  Admin permissions:");
    for perm in admin.permissions.get_all_permissions() {
        println!("    - {}", perm);
    }

    println!();
    println!("  Editor permissions:");
    for perm in editor.permissions.get_all_permissions() {
        println!("    - {}", perm);
    }

    println!();
    println!(
        "  Admin has 'users.manage': {}",
        admin.has_permission("users.manage")
    );
    println!(
        "  Editor has 'posts.create': {}",
        editor.has_permission("posts.create")
    );
    println!(
        "  Editor has 'users.manage': {}",
        editor.has_permission("users.manage")
    );
}

fn combined_example() {
    // Setup
    let mut gate = Gate::new();
    let mut registry = PolicyRegistry::new();

    // Define gates
    gate.define(
        "publish-post",
        Arc::new(|user: &User, _| user.has_all_permissions(&["posts.create", "posts.update"])),
    );

    // Register policy
    registry.register::<Post, PostPolicy>(PostPolicy);

    let admin = User::new_admin(1);
    let editor = User::new_editor(2);

    let post = Post {
        id: 1,
        author_id: 2,
        published: true,
    };

    println!(
        "  Admin can publish post (gate): {}",
        gate.allows(&admin, "publish-post")
    );
    println!(
        "  Admin can update post (policy): {}",
        registry.can(&admin, "update", Some(&post))
    );
    println!();
    println!(
        "  Editor can publish post (gate): {}",
        gate.allows(&editor, "publish-post")
    );
    println!(
        "  Editor can update own post (policy): {}",
        registry.can(&editor, "update", Some(&post))
    );
    println!(
        "  Editor can delete post (policy): {}",
        registry.cannot(&editor, "delete", Some(&post))
    );

    // Use authorize to throw errors
    match registry.authorize(&editor, "delete", Some(&post)) {
        Ok(()) => println!("  Editor authorized to delete"),
        Err(e) => println!("  Editor NOT authorized to delete: {}", e),
    }
}
