# Authorization System - Usage Guide

## Quick Start

### 1. Basic Gate Usage

Gates are perfect for simple, closure-based permission checks:

```rust
use rf_authorization::gates::Gate;
use std::sync::Arc;

// Create a gate
let mut gate = Gate::new();

// Define abilities
gate.define("create-post", Arc::new(|user: &User, _| {
    user.is_admin || user.has_permission("create-post")
}));

gate.define("delete-post", Arc::new(|user: &User, _| {
    user.is_admin
}));

// Check permissions
if gate.allows(&user, "create-post") {
    // User is allowed
}

// Or throw an error
gate.authorize(&user, "delete-post")?;
```

### 2. Policy-Based Authorization

Policies organize authorization logic around models:

```rust
use rf_authorization::policies::{Policy, PolicyRegistry};

// Define a policy
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
}

// Register and use
let mut registry = PolicyRegistry::new();
registry.register::<Post, PostPolicy>(PostPolicy);

// Check authorization
if registry.can(&user, "update", Some(&post)) {
    // Allow update
}

// Or throw error
registry.authorize(&user, "delete", Some(&post))?;
```

### 3. Database-Backed Permissions (RBAC)

Full role-based access control:

```rust
use rf_authorization::permissions::{Permission, Role, UserPermissions, HasPermissions};

// Define roles
let admin_role = Role::new(1, "admin")
    .with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(2, "posts.delete"),
        Permission::new(3, "users.manage"),
    ]);

let editor_role = Role::new(2, "editor")
    .with_permissions(vec![
        Permission::new(1, "posts.create"),
        Permission::new(4, "posts.update"),
    ]);

// Create user permissions
let user_permissions = UserPermissions::from_roles(vec![admin_role, editor_role]);

// Check permissions
assert!(user_permissions.has("posts.create"));
assert!(user_permissions.has_all(&["posts.create", "posts.delete"]));
assert!(user_permissions.has_any(&["posts.delete", "users.manage"]));
assert!(user_permissions.has_role("admin"));

// Implement on your User model
struct User {
    id: i64,
    permissions: UserPermissions,
}

impl HasPermissions for User {
    fn get_permissions(&self) -> &UserPermissions {
        &self.permissions
    }
}

// Now you can use convenience methods
if user.has_permission("posts.create") {
    // Create post
}
```

### 4. Middleware for Route Protection

```rust
use rf_authorization::middleware::{AuthorizeGateMiddleware, Middleware, Request};
use std::sync::Arc;

// Create middleware
let middleware = AuthorizeGateMiddleware::new(Arc::new(gate), "admin");

// Handle request
let request = Request::new().with_user(user);
let response = middleware.handle(request).await?;
```

## Advanced Usage

### Combining Gates and Policies

```rust
// Use gates for simple checks
gate.define("publish-post", Arc::new(|user: &User, _| {
    user.has_all_permissions(&["posts.create", "posts.publish"])
}));

// Use policies for model-specific checks
registry.authorize(&user, "update", Some(&post))?;

// Combine both in your handler
if gate.allows(&user, "publish-post") && registry.can(&user, "update", Some(&post)) {
    // Publish the post
}
```

### Multiple Roles

```rust
let user_permissions = UserPermissions::from_roles(vec![
    admin_role,
    editor_role,
    moderator_role,
]);

// Permissions from all roles are combined and deduplicated
assert!(user_permissions.has("admin.permission"));
assert!(user_permissions.has("editor.permission"));
assert!(user_permissions.has("moderator.permission"));
```

### Batch Permission Checks

```rust
// Check multiple abilities (must have ALL)
if gate.allows_all(&user, &["read", "write", "execute"]) {
    // User has all permissions
}

// Check multiple abilities (must have ANY)
if gate.allows_any(&user, &["admin", "superuser"]) {
    // User has at least one permission
}
```

## Database Integration

### Loading User Permissions from Database

```rust
use rf_authorization::permissions::PermissionLoader;
use async_trait::async_trait;

struct DatabasePermissionLoader {
    db: DatabaseConnection,
}

#[async_trait]
impl PermissionLoader for DatabasePermissionLoader {
    async fn load_user_permissions(&self, user_id: i64) -> Result<Vec<Permission>, String> {
        // Load from database
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT DISTINCT p.id, p.name, p.description
            FROM permissions p
            INNER JOIN role_permissions rp ON p.id = rp.permission_id
            INNER JOIN user_roles ur ON rp.role_id = ur.role_id
            WHERE ur.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(permissions)
    }

    async fn load_user_roles(&self, user_id: i64) -> Result<Vec<Role>, String> {
        // Load roles with their permissions
        let roles = sqlx::query!(
            r#"
            SELECT r.id, r.name, r.description
            FROM roles r
            INNER JOIN user_roles ur ON r.id = ur.role_id
            WHERE ur.user_id = $1
            "#,
            user_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| e.to_string())?;

        let mut result_roles = Vec::new();
        for role_data in roles {
            let permissions = self.load_role_permissions(role_data.id).await?;
            result_roles.push(
                Role::new(role_data.id, role_data.name)
                    .with_description(role_data.description.unwrap_or_default())
                    .with_permissions(permissions)
            );
        }

        Ok(result_roles)
    }

    async fn load_role_permissions(&self, role_id: i64) -> Result<Vec<Permission>, String> {
        let permissions = sqlx::query_as!(
            Permission,
            r#"
            SELECT p.id, p.name, p.description
            FROM permissions p
            INNER JOIN role_permissions rp ON p.id = rp.permission_id
            WHERE rp.role_id = $1
            "#,
            role_id
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| e.to_string())?;

        Ok(permissions)
    }
}

// Use in authentication
async fn load_authenticated_user(db: &Database, user_id: i64) -> User {
    let loader = DatabasePermissionLoader { db: db.clone() };
    let roles = loader.load_user_roles(user_id).await.unwrap();

    User {
        id: user_id,
        permissions: UserPermissions::from_roles(roles),
    }
}
```

### Recommended Database Schema

```sql
-- Permissions table
CREATE TABLE permissions (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Roles table
CREATE TABLE roles (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Role-Permission junction table
CREATE TABLE role_permissions (
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id BIGINT NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, permission_id)
);

-- User-Role junction table
CREATE TABLE user_roles (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, role_id)
);

-- Indexes for performance
CREATE INDEX idx_role_permissions_role ON role_permissions(role_id);
CREATE INDEX idx_role_permissions_permission ON role_permissions(permission_id);
CREATE INDEX idx_user_roles_user ON user_roles(user_id);
CREATE INDEX idx_user_roles_role ON user_roles(role_id);
```

### Seeding Initial Data

```sql
-- Insert basic permissions
INSERT INTO permissions (name, description) VALUES
    ('posts.view', 'View posts'),
    ('posts.create', 'Create posts'),
    ('posts.update', 'Update posts'),
    ('posts.delete', 'Delete posts'),
    ('posts.publish', 'Publish posts'),
    ('users.view', 'View users'),
    ('users.create', 'Create users'),
    ('users.update', 'Update users'),
    ('users.delete', 'Delete users'),
    ('roles.manage', 'Manage roles and permissions');

-- Insert roles
INSERT INTO roles (name, description) VALUES
    ('admin', 'Full system access'),
    ('editor', 'Can create and edit content'),
    ('author', 'Can create own content'),
    ('viewer', 'Read-only access');

-- Assign permissions to admin role (all permissions)
INSERT INTO role_permissions (role_id, permission_id)
SELECT 1, id FROM permissions;

-- Assign permissions to editor role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 2, id FROM permissions WHERE name IN (
    'posts.view', 'posts.create', 'posts.update', 'posts.publish',
    'users.view'
);

-- Assign permissions to author role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 3, id FROM permissions WHERE name IN (
    'posts.view', 'posts.create', 'posts.update'
);

-- Assign permissions to viewer role
INSERT INTO role_permissions (role_id, permission_id)
SELECT 4, id FROM permissions WHERE name IN (
    'posts.view', 'users.view'
);
```

## Best Practices

### 1. Permission Naming Convention

Use a hierarchical naming scheme:
- `resource.action` (e.g., `posts.create`, `users.delete`)
- `module.resource.action` (e.g., `admin.users.manage`)
- Keep names lowercase with underscores or dots

### 2. Gate vs Policy

**Use Gates when:**
- Simple permission check
- Not tied to a specific model
- Quick authorization decision
- Example: `gate.allows(&user, "admin")`

**Use Policies when:**
- Authorization depends on model state
- CRUD operations on models
- Complex business logic
- Example: `registry.can(&user, "update", Some(&post))`

### 3. Caching Permissions

```rust
// Load permissions once during authentication
let user_permissions = load_user_permissions(&db, user_id).await?;

// Store in session or JWT token (if small enough)
session.insert("permissions", user_permissions)?;

// Or cache in memory
cache.set(format!("user:{}:permissions", user_id), user_permissions, 3600)?;
```

### 4. Error Handling

```rust
match gate.authorize(&user, "admin") {
    Ok(()) => {
        // Proceed with action
    },
    Err(AuthorizationError::Forbidden(msg)) => {
        return Err(AppError::Forbidden(msg));
    },
    Err(e) => {
        return Err(AppError::Internal(e.to_string()));
    }
}
```

### 5. Testing

```rust
#[test]
fn test_admin_can_delete() {
    let mut gate = Gate::new();
    gate.define("delete-post", Arc::new(|user: &User, _| user.is_admin));

    let admin = User { is_admin: true, ..Default::default() };
    let regular = User { is_admin: false, ..Default::default() };

    assert!(gate.allows(&admin, "delete-post"));
    assert!(gate.denies(&regular, "delete-post"));
}
```

## Common Patterns

### Pattern 1: Super Admin Bypass

```rust
impl Policy<Post> for PostPolicy {
    type User = User;

    fn delete(&self, user: &User, _post: &Post) -> bool {
        // Super admin can do anything
        if user.is_super_admin {
            return true;
        }

        // Otherwise check specific permissions
        user.has_permission("posts.delete")
    }
}
```

### Pattern 2: Owner Check

```rust
impl Policy<Post> for PostPolicy {
    type User = User;

    fn update(&self, user: &User, post: &Post) -> bool {
        // Owner can always update
        if user.id == post.author_id {
            return true;
        }

        // Editors with permission can update
        user.has_permission("posts.update.any")
    }
}
```

### Pattern 3: Time-Based Permissions

```rust
gate.define("weekend-admin", Arc::new(|user: &User, _| {
    if user.has_permission("weekend.admin") {
        let now = chrono::Utc::now();
        let weekday = now.weekday();
        matches!(weekday, Weekday::Sat | Weekday::Sun)
    } else {
        false
    }
}));
```

## Examples

See `examples/basic_usage.rs` for a complete working example.

Run with:
```bash
cargo run --example basic_usage
```

## Testing

Run all tests:
```bash
cargo test
```

Run specific tests:
```bash
cargo test --test gates_test
cargo test --test policies_test
cargo test --test integration_test
```
