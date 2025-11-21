# Phase 13: Views, Authorization, Testing Tools

## Overview

Phase 13 implements critical features for web application support, focusing on template rendering, authorization, testing utilities, and enhanced email capabilities. This phase brings RustForge to full-stack web application readiness with Laravel-quality developer experience.

## Goals

1. **Blade-like Template System**: Tera-based view rendering with layouts and components
2. **Authorization System**: Policies and Gates for fine-grained access control
3. **Model Factories**: Laravel-style factories for testing
4. **Database Seeders**: Production data seeding
5. **Mailable Classes**: Enhanced email with templates

## Part 1: Template System (rf-view)

### 1.1 Tera Integration

Laravel's Blade equivalent using Tera template engine:

```rust
use rf_view::{View, ViewEngine};

// Render a view
let html = View::make("welcome", data)
    .render()
    .await?;

// With layout
let html = View::make("pages.home", data)
    .layout("layouts.app")
    .render()
    .await?;

// Response helper
async fn index() -> impl IntoResponse {
    View::make("home", json!({
        "title": "Welcome",
        "user": current_user()
    }))
}
```

### 1.2 Template Features

```html
{{!-- layouts/app.tera --}}
<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}{{ title }}{% endblock %}</title>
</head>
<body>
    {% include "partials.header" %}

    {% block content %}{% endblock %}

    {% include "partials.footer" %}
</body>
</html>

{{!-- pages/home.tera --}}
{% extends "layouts.app" %}

{% block title %}Home - {{ super() }}{% endblock %}

{% block content %}
<h1>Welcome, {{ user.name }}!</h1>

{% if posts %}
    {% for post in posts %}
    <article>
        <h2>{{ post.title }}</h2>
        <p>{{ post.excerpt }}</p>
    </article>
    {% endfor %}
{% else %}
    <p>No posts yet.</p>
{% endif %}
{% endblock %}
```

### 1.3 Custom Filters & Functions

```rust
// Register custom filters
ViewEngine::register_filter("currency", |value: &Value, _: &HashMap| {
    format!("${:.2}", value.as_f64().unwrap_or(0.0))
});

ViewEngine::register_function("route", |args: &HashMap| {
    route::url(args.get("name")?, args.get("params")?)
});

// Usage in templates
{{ price | currency }}
<a href="{{ route(name='post.show', params={id: post.id}) }}">Read more</a>
```

## Part 2: Authorization System

### 2.1 Policies

Laravel-style authorization policies:

```rust
use rf_authorization::{Policy, Authorizable};

pub struct PostPolicy;

impl Policy<User, Post> for PostPolicy {
    fn view(&self, user: Option<&User>, post: &Post) -> bool {
        post.published || user.map(|u| u.id == post.user_id).unwrap_or(false)
    }

    fn create(&self, user: &User) -> bool {
        user.is_verified()
    }

    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.user_id || user.is_admin()
    }

    fn delete(&self, user: &User, post: &Post) -> bool {
        user.id == post.user_id || user.is_admin()
    }
}

// Register policy
AuthorizationService::register::<Post, PostPolicy>();

// Usage in controllers
async fn update(post_id: i32, user: User) -> Result<Response> {
    let post = Post::find(post_id).await?;

    user.authorize("update", &post)?;

    // Update post...
    Ok(Response::ok())
}
```

### 2.2 Gates

Simple closure-based authorization:

```rust
use rf_authorization::Gate;

// Define gates
Gate::define("edit-settings", |user: &User| {
    user.is_admin()
});

Gate::define("view-dashboard", |user: &User| {
    user.has_permission("dashboard.view")
});

// Usage
if Gate::allows("edit-settings", &user) {
    // Show settings
}

Gate::authorize("view-dashboard", &user)?; // Throws if denied

// In templates
{% if gate.allows('edit-settings') %}
    <a href="{{ route('settings') }}">Settings</a>
{% endif %}
```

### 2.3 Middleware

```rust
use rf_authorization::middleware::{Authorize, Can};

// Protect routes
Router::new()
    .route("/admin/*", get(admin_dashboard))
    .layer(Authorize::gate("admin-access"))
    .route("/posts/:id/edit", get(edit_post))
    .layer(Can::new("update", Post::class()));
```

## Part 3: Model Factories

### 3.1 Factory Definitions

```rust
use rf_testing::{Factory, Faker};

#[derive(Factory)]
pub struct UserFactory;

impl FactoryDefinition for UserFactory {
    type Model = User;

    fn definition(faker: &Faker) -> Self::Model {
        User {
            id: 0,
            name: faker.name(),
            email: faker.email(),
            password: faker.password(12),
            email_verified_at: Some(faker.date_time()),
            created_at: faker.date_time(),
            updated_at: faker.date_time(),
        }
    }

    fn states() -> Vec<State<Self::Model>> {
        vec![
            State::new("unverified", |user| {
                user.email_verified_at = None;
                user
            }),
            State::new("admin", |user| {
                user.is_admin = true;
                user
            }),
        ]
    }
}

// Usage
let user = UserFactory::new().create().await?;
let admin = UserFactory::new().state("admin").create().await?;
let users = UserFactory::new().count(10).create().await?;

// With relationships
let post = PostFactory::new()
    .for_user(&user)
    .create()
    .await?;
```

### 3.2 Advanced Factory Features

```rust
// Custom attributes
let user = UserFactory::new()
    .with_email("specific@example.com")
    .with_name("John Doe")
    .create()
    .await?;

// Sequences
let users = UserFactory::new()
    .sequence("email", |n| format!("user{}@example.com", n))
    .count(10)
    .create()
    .await?;

// Callbacks
let user = UserFactory::new()
    .after_creating(|user, db| async move {
        Profile::create_for_user(user.id, db).await?;
        Ok(())
    })
    .create()
    .await?;
```

## Part 4: Database Seeders

### 4.1 Seeder Classes

```rust
use rf_testing::{Seeder, DatabaseSeeder};

pub struct DatabaseSeeder;

impl Seeder for DatabaseSeeder {
    async fn run(&self, db: &DatabaseConnection) -> Result<()> {
        UserSeeder.run(db).await?;
        PostSeeder.run(db).await?;
        CategorySeeder.run(db).await?;

        Ok(())
    }
}

pub struct UserSeeder;

impl Seeder for UserSeeder {
    async fn run(&self, db: &DatabaseConnection) -> Result<()> {
        // Create admin
        let admin = UserFactory::new()
            .state("admin")
            .with_email("admin@example.com")
            .create_in(db)
            .await?;

        // Create regular users
        UserFactory::new()
            .count(50)
            .create_in(db)
            .await?;

        Ok(())
    }
}
```

### 4.2 Seeder Commands

```bash
# Run all seeders
forge db:seed

# Run specific seeder
forge db:seed --class=UserSeeder

# Fresh database with seed
forge migrate:fresh --seed
```

## Part 5: Enhanced Mailable Classes

### 5.1 Mailable with Templates

```rust
use rf_mail::{Mailable, Mail};

pub struct WelcomeEmail {
    user: User,
}

impl Mailable for WelcomeEmail {
    fn build(&self) -> Mail {
        Mail::new()
            .to(&self.user.email)
            .subject("Welcome to RustForge!")
            .view("emails.welcome", json!({
                "user": self.user,
                "app_name": env!("APP_NAME"),
            }))
            .text("emails.welcome_plain")
    }
}

// Send
WelcomeEmail { user }.send().await?;
```

### 5.2 Email Templates

```html
{{!-- emails/welcome.tera --}}
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: Arial, sans-serif; }
        .button { background: #3490dc; color: white; padding: 10px 20px; }
    </style>
</head>
<body>
    <h1>Welcome, {{ user.name }}!</h1>

    <p>Thank you for joining {{ app_name }}.</p>

    <a href="{{ url }}" class="button">Get Started</a>

    <p>Best regards,<br>The {{ app_name }} Team</p>
</body>
</html>

{{!-- emails/welcome_plain.tera --}}
Welcome, {{ user.name }}!

Thank you for joining {{ app_name }}.

Get Started: {{ url }}

Best regards,
The {{ app_name }} Team
```

### 5.3 Markdown Mail

```rust
pub struct InvoicePaid {
    invoice: Invoice,
}

impl Mailable for InvoicePaid {
    fn build(&self) -> Mail {
        Mail::new()
            .to(&self.invoice.customer_email)
            .subject("Invoice Paid")
            .markdown("emails.invoice-paid", json!({
                "invoice": self.invoice,
            }))
    }
}
```

```markdown
{{!-- emails/invoice-paid.md --}}
# Invoice Paid

Hi {{ invoice.customer_name }},

Your invoice #{{ invoice.number }} has been paid.

**Amount:** ${{ invoice.amount }}
**Date:** {{ invoice.paid_at }}

@component('mail::button', url: invoice.url)
View Invoice
@endcomponent

Thanks,
{{ app_name }}
```

## Implementation Structure

### New Crates

```
crates/
├── rf-view/                  # NEW: Template system
│   ├── src/
│   │   ├── lib.rs
│   │   ├── engine.rs         # Tera engine wrapper
│   │   ├── view.rs           # View struct
│   │   ├── response.rs       # Axum response integration
│   │   └── macros.rs         # View macros
│   └── Cargo.toml
│
├── rf-authorization/         # NEW: Authorization
│   ├── src/
│   │   ├── lib.rs
│   │   ├── policy.rs         # Policy trait
│   │   ├── gate.rs           # Gate system
│   │   ├── authorizable.rs   # Authorizable trait
│   │   └── middleware.rs     # Auth middleware
│   └── Cargo.toml
│
├── rf-testing/               # ENHANCED
│   ├── src/
│   │   ├── factory.rs        # ENHANCED: Full factory system
│   │   ├── seeder.rs         # ENHANCED: Seeder system
│   │   ├── faker.rs          # NEW: Faker integration
│   │   └── database.rs       # NEW: Test database helpers
│   └── Cargo.toml
│
└── rf-mail/                  # ENHANCED
    ├── src/
    │   ├── mailable.rs       # ENHANCED: Template support
    │   ├── markdown.rs       # NEW: Markdown mail
    │   └── templates/        # NEW: Built-in templates
    └── Cargo.toml
```

## Laravel Feature Comparison

| Feature | Laravel | RustForge Phase 13 | Status |
|---------|---------|-------------------|--------|
| **Templates** |
| Blade Templates | ✅ | Tera Templates ✅ | Implementing |
| Layouts | ✅ | ✅ | Implementing |
| Components | ✅ | ✅ | Implementing |
| Slots | ✅ | ✅ | Implementing |
| Includes | ✅ | ✅ | Implementing |
| **Authorization** |
| Policies | ✅ | ✅ | Implementing |
| Gates | ✅ | ✅ | Implementing |
| Middleware | ✅ | ✅ | Implementing |
| **Testing** |
| Factories | ✅ | ✅ | Implementing |
| States | ✅ | ✅ | Implementing |
| Seeders | ✅ | ✅ | Implementing |
| Faker | ✅ | ✅ | Implementing |
| **Mail** |
| Mailable Classes | ✅ | ✅ | Implementing |
| Mail Templates | ✅ | ✅ | Implementing |
| Markdown Mail | ✅ | ✅ | Implementing |

## Dependencies

```toml
# rf-view
[dependencies]
tera = "1.19"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
axum = "0.7"

# rf-authorization
[dependencies]
async-trait = "0.1"
thiserror = "1.0"

# rf-testing (additions)
[dependencies]
fake = "2.9"
rand = "0.8"

# rf-mail (additions)
[dependencies]
tera = "1.19"
comrak = "0.20"  # Markdown processing
```

## Success Criteria

- [ ] Tera templates render with layouts
- [ ] View responses work in Axum routes
- [ ] Policies authorize actions correctly
- [ ] Gates work in templates and code
- [ ] Factories create test data
- [ ] Seeders populate database
- [ ] Mailable classes send templated emails
- [ ] All features have >90% test coverage
- [ ] Complete documentation with examples

## Timeline

- **Part 1: Templates** (3-4 days)
- **Part 2: Authorization** (2-3 days)
- **Part 3: Factories** (2-3 days)
- **Part 4: Seeders** (1-2 days)
- **Part 5: Mailable Enhancement** (1-2 days)
- **Testing & Documentation** (2 days)

**Total: ~2-3 weeks**

## Notes

- Tera is Rust's closest equivalent to Blade (Jinja2-like syntax)
- Authorization integrates with existing rf-auth
- Factories build on existing rf-testing
- Mailable enhances existing rf-mail
- All features designed for Laravel developer familiarity
