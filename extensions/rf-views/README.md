# rf-views

A powerful Blade-like template system for RustForge built on [Tera](https://tera.netlify.app/).

## Features

- **Template Inheritance**: Use layouts and sections like Laravel Blade
- **Custom Filters**: Route generation, asset URLs, date/money formatting, and more
- **Custom Functions**: CSRF tokens, authentication, validation errors, flash messages
- **Components**: Reusable UI components with a simple registration API
- **Axum Integration**: First-class support for the Axum web framework
- **Testing Utilities**: Comprehensive testing helpers for your templates
- **Type-Safe**: Full Rust type safety with serializable data structures

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-views = { path = "../rf-views" }
```

### Basic Usage

```rust
use rf_views::prelude::*;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a view engine
    let engine = ViewEngine::new("resources/views")?;

    // Render a template
    let html = engine.render_with_data("welcome", json!({
        "name": "World",
        "title": "Welcome Page"
    }))?;

    println!("{}", html);
    Ok(())
}
```

### With Axum

```rust
use axum::{Router, routing::get, extract::State};
use rf_views::prelude::*;
use std::sync::Arc;

async fn index(State(engine): State<Arc<ViewEngine>>)
    -> Result<axum::response::Html<String>, ViewError>
{
    view(&engine, "index", serde_json::json!({
        "title": "Home"
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Arc::new(ViewEngine::new("resources/views")?);

    let app = Router::new()
        .route("/", get(index))
        .with_state(engine);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

## Template Syntax

### Layouts

Create a base layout in `layouts/app.tera`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>{% block title %}App{% endblock %}</title>
</head>
<body>
    {% include "partials/nav" %}

    {% if flash(key='success') %}
        <div class="alert alert-success">{{ flash(key='success') }}</div>
    {% endif %}

    {% block content %}{% endblock %}
</body>
</html>
```

### Views

Extend the layout in `posts/index.tera`:

```html
{% extends "layouts/app" %}

{% block title %}Posts{% endblock %}

{% block content %}
    <h1>All Posts</h1>

    {% for post in posts %}
        <article>
            <h2>{{ post.title }}</h2>
            <p>{{ post.body | truncate(length=200) }}</p>
            <small>By {{ post.user.name }} on {{ post.created_at | date(format="%B %d, %Y") }}</small>
        </article>
    {% endfor %}
{% endblock %}
```

### Forms with Validation

```html
<form method="POST" action="/posts">
    <input type="hidden" name="csrf_token" value="{{ csrf_token() }}">

    <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" value="{{ old(key='title') }}">
        {% if error(field='title') %}
            <span class="error">{{ error(field='title') }}</span>
        {% endif %}
    </div>

    <button type="submit">Create Post</button>
</form>
```

## Built-in Functions

### Authentication

```html
{% if auth() %}
    <p>Welcome, {{ auth().name }}!</p>
{% else %}
    <a href="/login">Login</a>
{% endif %}
```

### CSRF Protection

```html
<input type="hidden" name="csrf_token" value="{{ csrf_token() }}">
```

### Flash Messages

```html
{% if flash(key='success') %}
    <div class="alert alert-success">{{ flash(key='success') }}</div>
{% endif %}
```

### Validation Errors

```html
{% if error(field='email') %}
    <span class="error">{{ error(field='email') }}</span>
{% endif %}

<!-- Check if field has errors -->
{% if has_error(field='email') %}
    <div class="field-error">Please check the email field</div>
{% endif %}

<!-- Get all errors for a field -->
{% for err in errors(field='email') %}
    <li>{{ err }}</li>
{% endfor %}
```

### Old Input

```html
<input type="text" name="email" value="{{ old(key='email') }}">
```

## Built-in Filters

### Date Formatting

```html
{{ post.created_at | date(format="%B %d, %Y") }}
<!-- Output: January 01, 2025 -->
```

### Money Formatting

```html
{{ product.price | money(currency="USD") }}
<!-- Output: $42.50 -->
```

### Text Truncation

```html
{{ post.body | truncate(length=100, suffix="...") }}
```

### Pluralization

```html
{{ count | pluralize(singular="item", plural="items") }}
<!-- Output: "5 items" or "1 item" -->
```

## Components

### Register Components

```rust
use rf_views::prelude::*;

let registry = ComponentRegistry::new();

// Register built-in components
register_default_components(&registry)?;

// Register custom component
registry.register("custom_card", |context: &Context| {
    let title = context.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Card");

    Ok(format!(r#"
        <div class="card">
            <h3>{}</h3>
            {}
        </div>
    "#, title, context.get("content").and_then(|v| v.as_str()).unwrap_or("")))
})?;
```

### Use Components in Templates

```html
<!-- Alert component -->
{{ component(name="alert", type="success", message="Operation successful!") }}

<!-- Button component -->
{{ component(name="button", text="Click me", variant="primary", type="submit") }}

<!-- Input component -->
{{ component(name="input", name="email", type="email", label="Email Address", required=true) }}
```

## Helper Functions

### View Helpers

```rust
use rf_views::helpers::*;

// Simple view rendering
let html = view(&engine, "posts.index", &posts)?;

// View with context
let context = context! {
    "posts" => posts,
    "title" => "All Posts"
};
let html = view_with_context(&engine, "posts.index", &context)?;
```

### Redirect Helpers

```rust
// Redirect with flash message
return Ok(redirect_with_success(&engine, "/posts", "Post created!"));

// Redirect with error
return Ok(redirect_with_error(&engine, "/posts", "Failed to create post"));

// Other flash types
redirect_with_info(&engine, "/", "Welcome back!");
redirect_with_warning(&engine, "/", "Please verify your email");
```

### View Builder

```rust
use rf_views::helpers::ViewBuilder;

let response = ViewBuilder::new(engine.clone(), "posts.show")
    .with("post", post)
    .with("comments", comments)
    .status(StatusCode::OK)
    .into_response();
```

## Testing

### Test Utilities

```rust
use rf_views::testing::*;

#[test]
fn test_post_template() {
    let engine = create_test_engine_with_templates(vec![
        ("test", "Hello {{ name }}!")
    ]).unwrap();

    // Assert template exists
    assert!(assert_view_exists(&engine, "test"));

    // Assert template renders
    let html = assert_view_renders(&engine, "test",
        serde_json::json!({"name": "World"})).unwrap();
    assert_eq!(html, "Hello World!");

    // Assert template contains text
    assert!(assert_view_contains(&engine, "test",
        serde_json::json!({"name": "World"}), "World"));
}
```

### Test Builder

```rust
let builder = TestViewBuilder::new(engine);

// Render and assert
builder.assert_contains("posts.index", &posts, "Posts").unwrap();
builder.assert_output("welcome", &data, "Welcome!").unwrap();
```

### Snapshot Testing

```rust
let snapshot = ViewSnapshot::new(
    "posts.show",
    serde_json::json!({"post": post}),
    expected_html
);

snapshot.verify(&engine).unwrap();
```

## Configuration

```rust
use rf_views::{ViewEngine, ViewConfig};

let config = ViewConfig::new("templates")
    .cache_enabled(true)
    .auto_reload(true)  // Reload templates in development
    .strict_mode(false) // Don't fail on missing variables
    .extension("html"); // Use .html instead of .tera

let engine = ViewEngine::with_config(config)?;
```

## Examples

See the `examples/` directory for complete working examples:

- `basic.rs` - Basic template rendering
- `axum_integration.rs` - Full Axum web application
- `views/` - Example templates (layouts, posts, forms)

Run examples:

```bash
cargo run --example basic
cargo run --example axum_integration
```

## Template Directory Structure

```
resources/views/
├── layouts/
│   └── app.tera
├── partials/
│   ├── nav.tera
│   └── footer.tera
├── posts/
│   ├── index.tera
│   ├── show.tera
│   ├── create.tera
│   └── edit.tera
└── components/
    ├── alert.tera
    └── card.tera
```

## Error Handling

```rust
use rf_views::{ViewError, ViewResult};

fn render_post(engine: &ViewEngine, post: &Post) -> ViewResult<String> {
    engine.render_with_data("posts.show", post)
}

match render_post(&engine, &post) {
    Ok(html) => println!("{}", html),
    Err(ViewError::TemplateNotFound(name)) => {
        eprintln!("Template not found: {}", name);
    }
    Err(ViewError::RenderError(msg)) => {
        eprintln!("Render error: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Advanced Features

### Custom Filters

```rust
use tera::{Filter, Value};

struct UppercaseFilter;

impl Filter for UppercaseFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>)
        -> Result<Value, tera::Error>
    {
        let text = value.as_str().ok_or_else(||
            tera::Error::msg("Value must be a string"))?;
        Ok(Value::String(text.to_uppercase()))
    }
}

let mut engine = ViewEngine::new("views")?;
engine.add_filter("uppercase", UppercaseFilter)?;
```

### Custom Functions

```rust
use tera::{Function, Value};

struct CurrentYearFunction;

impl Function for CurrentYearFunction {
    fn call(&self, _args: &HashMap<String, Value>)
        -> Result<Value, tera::Error>
    {
        Ok(Value::Number(2025.into()))
    }
}

engine.add_function("current_year", CurrentYearFunction)?;
```

## Performance

- Template compilation is cached by default
- Auto-reload can be disabled in production for better performance
- Use `cache_enabled(true)` for optimal performance

## License

MIT OR Apache-2.0
