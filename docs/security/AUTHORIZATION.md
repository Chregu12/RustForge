# Authorization Guide (Gates & Policies)

## Overview

Foundry provides two complementary authorization systems:
- **Gates**: General ability checks (e.g., "can manage users")
- **Policies**: Resource-specific permissions (e.g., "can edit this post")

## Quick Start

### Gates

```rust
use foundry_application::auth::{Gate, AuthorizationError};

// Define a gate
Gate::define("manage-users", |args| {
    let user = args.downcast_ref::<User>().unwrap();
    user.is_admin()
}).await;

// Check authorization
if Gate::allows("manage-users", &user).await {
    // User can manage users
}

// Or authorize (throws error if denied)
Gate::authorize("manage-users", &user).await?;
```

### Policies

```rust
use foundry_application::auth::{Policy, ResourcePolicy};

struct PostPolicy;

impl ResourcePolicy<User, Post> for PostPolicy {
    fn view(&self, user: &User, post: &Post) -> bool {
        post.is_published || user.id == post.author_id
    }

    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id || user.is_admin()
    }

    fn delete(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id || user.is_admin()
    }
}

// Register policy
Policy::register(PostPolicy).await;

// Check authorization
if Policy::allows("update", &user, &post).await {
    // User can update this post
}
```

---

## Gates

### Defining Gates

Gates are simple callbacks that return `true` or `false`:

```rust
// Simple gate
Gate::define("view-dashboard", |args| {
    let user = args.downcast_ref::<User>().unwrap();
    user.is_authenticated()
}).await;

// Gate with resource
Gate::define("edit-settings", |args| {
    let (user, settings) = args.downcast_ref::<(User, Settings)>().unwrap();
    user.id == settings.owner_id || user.is_admin()
}).await;

// Complex logic
Gate::define("manage-billing", |args| {
    let user = args.downcast_ref::<User>().unwrap();
    user.has_permission("billing.manage") &&
    user.email_verified &&
    !user.is_suspended()
}).await;
```

### Checking Gates

```rust
// Simple check
if Gate::allows("view-dashboard", &user).await {
    // Allowed
}

// Deny check
if Gate::denies("view-dashboard", &user).await {
    // Denied
}

// Authorize (throws error if denied)
Gate::authorize("view-dashboard", &user).await?;
```

### Before Hooks (Super Admin)

Before hooks run before all gate checks:

```rust
// Allow super admins to bypass all gates
Gate::before(|args| {
    if let Some(user) = args.downcast_ref::<User>() {
        if user.is_super_admin() {
            return Some(true);  // Bypass all gates
        }
    }
    None  // Continue with normal gate check
}).await;
```

### After Hooks

After hooks can modify the result of gate checks:

```rust
// Log all authorization checks
Gate::after(|args, result| {
    if let Some(user) = args.downcast_ref::<User>() {
        tracing::info!("Authorization check for user {}: {}", user.id, result);
    }
    result  // Return original result
}).await;
```

### Use in Route Handlers

```rust
use axum::{extract::Path, http::StatusCode};
use foundry_application::auth::{Gate, RequireAuth};

async fn delete_user(
    RequireAuth(current_user): RequireAuth,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, AuthorizationError> {
    // Check if user can manage users
    Gate::authorize("manage-users", &current_user).await?;

    // Perform deletion
    User::delete(user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
```

---

## Policies

### Resource Policy Trait

Implement the `ResourcePolicy` trait for each model:

```rust
use foundry_application::auth::ResourcePolicy;

struct CommentPolicy;

impl ResourcePolicy<User, Comment> for CommentPolicy {
    fn view(&self, _user: &User, comment: &Comment) -> bool {
        !comment.is_deleted()
    }

    fn create(&self, user: &User) -> bool {
        user.is_authenticated() && !user.is_banned()
    }

    fn update(&self, user: &User, comment: &Comment) -> bool {
        user.id == comment.author_id && !comment.is_locked()
    }

    fn delete(&self, user: &User, comment: &Comment) -> bool {
        user.id == comment.author_id ||
        user.is_moderator() ||
        user.is_admin()
    }

    fn can(&self, action: &str, user: &User, comment: &Comment) -> bool {
        match action {
            "report" => user.is_authenticated(),
            "pin" => user.is_moderator(),
            "lock" => user.is_moderator() || user.is_admin(),
            _ => false
        }
    }
}

// Register the policy
Policy::register(CommentPolicy).await;
```

### Standard CRUD Actions

The `ResourcePolicy` trait provides these standard methods:
- `view()`: Can view the resource
- `create()`: Can create new resources
- `update()`: Can update this resource
- `delete()`: Can delete this resource

### Custom Actions

Use the `can()` method for custom actions:

```rust
impl ResourcePolicy<User, Post> for PostPolicy {
    fn can(&self, action: &str, user: &User, post: &Post) -> bool {
        match action {
            "publish" => user.id == post.author_id && post.is_draft(),
            "feature" => user.is_editor() || user.is_admin(),
            "archive" => user.is_admin(),
            _ => false
        }
    }
}

// Check custom action
if Policy::allows("publish", &user, &post).await {
    post.publish().await?;
}
```

### Before Hooks (Super Admin)

```rust
// Allow super admins to do anything
Policy::before(|user, _action, _resource| {
    if let Some(user) = user.downcast_ref::<User>() {
        if user.is_super_admin() {
            return Some(true);
        }
    }
    None
}).await;
```

### Use in Route Handlers

```rust
async fn update_post(
    RequireAuth(user): RequireAuth,
    Path(post_id): Path<i64>,
    Json(data): Json<UpdatePostRequest>,
) -> Result<Json<Post>, AuthorizationError> {
    let post = Post::find(post_id).await?;

    // Authorize update
    Policy::authorize("update", &user, &post).await?;

    // Perform update
    post.update(data).await?;

    Ok(Json(post))
}
```

---

## Combining Gates and Policies

Use gates for general abilities and policies for resource-specific permissions:

```rust
// Gate: Can user access admin panel?
Gate::define("access-admin", |args| {
    let user = args.downcast_ref::<User>().unwrap();
    user.is_admin() || user.is_moderator()
}).await;

// Policy: Can user edit this specific post?
impl ResourcePolicy<User, Post> for PostPolicy {
    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id || user.is_admin()
    }
}

// Usage
async fn admin_edit_post(
    RequireAuth(user): RequireAuth,
    Path(post_id): Path<i64>,
) -> Result<Json<Post>, AuthorizationError> {
    // First check general admin access
    Gate::authorize("access-admin", &user).await?;

    // Then check specific post permission
    let post = Post::find(post_id).await?;
    Policy::authorize("update", &user, &post).await?;

    // Proceed with edit
    Ok(Json(post))
}
```

---

## Common Patterns

### Owner or Admin

```rust
impl ResourcePolicy<User, Resource> for ResourcePolicy {
    fn update(&self, user: &User, resource: &Resource) -> bool {
        user.id == resource.owner_id || user.is_admin()
    }
}
```

### Team-Based Access

```rust
impl ResourcePolicy<User, Project> for ProjectPolicy {
    fn view(&self, user: &User, project: &Project) -> bool {
        project.is_public ||
        project.team_members.contains(&user.id) ||
        user.is_admin()
    }
}
```

### Role-Based Access

```rust
impl ResourcePolicy<User, Document> for DocumentPolicy {
    fn update(&self, user: &User, document: &Document) -> bool {
        match document.status {
            DocumentStatus::Draft => user.id == document.author_id,
            DocumentStatus::Review => user.has_role("reviewer"),
            DocumentStatus::Published => user.has_role("editor"),
        }
    }
}
```

### Time-Based Access

```rust
impl ResourcePolicy<User, Event> for EventPolicy {
    fn register(&self, user: &User, event: &Event) -> bool {
        let now = Utc::now();
        user.is_authenticated() &&
        event.registration_opens_at <= now &&
        event.registration_closes_at >= now &&
        event.available_seats > 0
    }
}
```

---

## Middleware Integration

### Require Authorization

Create middleware to enforce authorization:

```rust
use axum::{extract::State, middleware::Next};

async fn require_gate(
    State(gate_name): State<String>,
    RequireAuth(user): RequireAuth,
    request: Request,
    next: Next,
) -> Result<Response, AuthorizationError> {
    Gate::authorize(&gate_name, &user).await?;
    Ok(next.run(request).await)
}

// Usage
let app = Router::new()
    .route("/admin/*", get(admin_handler))
    .layer(axum::middleware::from_fn_with_state(
        "access-admin".to_string(),
        require_gate
    ));
```

---

## Testing

### Testing Gates

```rust
#[tokio::test]
async fn test_admin_gate() {
    Gate::define("admin-only", |args| {
        let user = args.downcast_ref::<User>().unwrap();
        user.is_admin()
    }).await;

    let admin = User { id: 1, role: Role::Admin };
    let regular_user = User { id: 2, role: Role::User };

    assert!(Gate::allows("admin-only", &admin).await);
    assert!(Gate::denies("admin-only", &regular_user).await);
}
```

### Testing Policies

```rust
#[tokio::test]
async fn test_post_policy() {
    Policy::register(PostPolicy).await;

    let author = User { id: 1 };
    let other_user = User { id: 2 };
    let post = Post { id: 1, author_id: 1 };

    // Author can update their own post
    assert!(Policy::allows("update", &author, &post).await);

    // Other users cannot
    assert!(Policy::denies("update", &other_user, &post).await);
}
```

---

## Best Practices

### ✅ DO
- Use gates for general abilities
- Use policies for resource-specific permissions
- Define before hooks for super admin bypass
- Keep authorization logic in policies, not controllers
- Test all authorization rules
- Use descriptive gate names (`manage-users` not `check1`)

### ❌ DON'T
- Duplicate authorization logic across controllers
- Put complex business logic in gates
- Forget to check authorization
- Use authorization as a substitute for validation
- Hard-code permissions in multiple places

---

## Performance Considerations

### Caching Results

For expensive authorization checks:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// Simple cache (in production, use proper caching)
static AUTH_CACHE: Lazy<Arc<RwLock<HashMap<String, bool>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

async fn cached_authorize(user: &User, post: &Post) -> bool {
    let key = format!("user:{}:post:{}", user.id, post.id);

    // Check cache
    {
        let cache = AUTH_CACHE.read().await;
        if let Some(&result) = cache.get(&key) {
            return result;
        }
    }

    // Compute authorization
    let result = Policy::allows("update", user, post).await;

    // Store in cache
    AUTH_CACHE.write().await.insert(key, result);

    result
}
```

---

## Related Documentation

- [Authentication](./AUTHENTICATION.md)
- [Roles & Permissions](./ROLES_PERMISSIONS.md)
- [Security Best Practices](./SECURITY_BEST_PRACTICES.md)
