# HTMX Integration Guide for RustForge

> **The Rust Alternative to Laravel Livewire**

## Overview

While Laravel uses Livewire for dynamic, reactive interfaces, RustForge recommends **htmx** as the idiomatic Rust alternative. htmx provides similar benefits without the complexity of WebSocket management or stateful server components.

### Why htmx for Rust?

**Architectural Advantages:**
- **Zero JavaScript framework** - Just HTML attributes
- **Stateless by design** - Perfect for Rust's ownership model
- **No WebSocket complexity** - Standard HTTP requests
- **Progressive enhancement** - Works without JS
- **Tiny footprint** - 14kb minified

**Laravel Livewire vs RustForge + htmx:**

| Feature | Laravel Livewire | RustForge + htmx |
|---------|------------------|------------------|
| Reactivity | WebSocket + PHP | HTTP + Rust |
| State Management | Server-side session | Stateless/REST |
| Bundle Size | ~60kb | ~14kb |
| Learning Curve | Medium | Low |
| Performance | Good | Excellent |
| Scalability | Moderate | Excellent |

## Installation

### 1. Add htmx to Your Project

**Via CDN (Quick Start):**

```html
<!-- In your layout template -->
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
```

**Via npm (Production):**

```bash
npm install htmx.org
```

```javascript
// In your main.js
import 'htmx.org';
```

### 2. Setup RustForge Routes

htmx works perfectly with standard Axum handlers:

```rust
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, Form},
    response::{Html, IntoResponse},
};
use serde::Deserialize;

async fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/users", get(users_index))
        .route("/users", post(users_create))
        .route("/users/:id", delete(users_delete))
        .route("/users/:id/edit", get(users_edit_form))
        .route("/users/:id", post(users_update))
}
```

## Core Patterns

### Pattern 1: Inline Editing

**Laravel Livewire:**
```php
<livewire:edit-user :user="$user" />
```

**RustForge + htmx:**

```rust
// Handler
async fn users_show(Path(id): Path(i64>) -> Html<String> {
    let user = get_user(id).await;
    Html(format!(r#"
        <div id="user-{id}">
            <h2>{}</h2>
            <button
                hx-get="/users/{id}/edit"
                hx-target="#user-{id}"
                hx-swap="outerHTML">
                Edit
            </button>
        </div>
    "#, user.name))
}

async fn users_edit_form(Path(id): Path<i64>) -> Html<String> {
    let user = get_user(id).await;
    Html(format!(r#"
        <form
            id="user-{id}"
            hx-post="/users/{id}"
            hx-target="#user-{id}"
            hx-swap="outerHTML">
            <input name="name" value="{}" />
            <button type="submit">Save</button>
            <button hx-get="/users/{id}" hx-target="#user-{id}">Cancel</button>
        </form>
    "#, user.name))
}

#[derive(Deserialize)]
struct UpdateUser {
    name: String,
}

async fn users_update(
    Path(id): Path<i64>,
    Form(data): Form<UpdateUser>
) -> Html<String> {
    update_user(id, data.name).await;
    users_show(Path(id)).await
}
```

### Pattern 2: Live Search

**Laravel Livewire:**
```php
<input wire:model.debounce.300ms="search">
```

**RustForge + htmx:**

```html
<input
    type="search"
    name="q"
    placeholder="Search users..."
    hx-get="/users/search"
    hx-trigger="keyup changed delay:300ms"
    hx-target="#search-results"
    hx-indicator=".spinner" />

<div class="spinner htmx-indicator">Searching...</div>
<div id="search-results"></div>
```

```rust
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn users_search(Query(query): Query<SearchQuery>) -> Html<String> {
    let users = search_users(&query.q).await;

    let results = users.iter()
        .map(|u| format!(r#"<div class="user">{}</div>"#, u.name))
        .collect::<Vec<_>>()
        .join("\n");

    Html(results)
}
```

### Pattern 3: Infinite Scroll

**Laravel Livewire:**
```php
<div wire:init="loadMore">
```

**RustForge + htmx:**

```html
<div id="users-list">
    <!-- Initial users -->
    {{#each users}}
    <div class="user">{{name}}</div>
    {{/each}}

    <div
        hx-get="/users?page={{next_page}}"
        hx-trigger="revealed"
        hx-swap="afterend">
        <span class="spinner">Loading more...</span>
    </div>
</div>
```

```rust
async fn users_paginated(Query(params): Query<PaginationParams>) -> Html<String> {
    let users = get_users_page(params.page).await;
    let next_page = params.page + 1;

    let mut html = users.iter()
        .map(|u| format!(r#"<div class="user">{}</div>"#, u.name))
        .collect::<Vec<_>>()
        .join("\n");

    if has_more_users(next_page).await {
        html.push_str(&format!(r#"
            <div hx-get="/users?page={next_page}"
                 hx-trigger="revealed"
                 hx-swap="afterend">
                <span class="spinner">Loading more...</span>
            </div>
        "#));
    }

    Html(html)
}
```

### Pattern 4: Real-time Validation

**RustForge + htmx + rf-validation:**

```rust
use rf_validation::{Validate, ValidationError};

#[derive(Deserialize, Validate)]
struct CreateUserForm {
    #[validate(length(min = 3, max = 50))]
    name: String,

    #[validate(email)]
    email: String,
}

async fn validate_field(
    Form(field): Form<FieldValidation>
) -> Html<String> {
    match field.name.as_str() {
        "email" => {
            if !is_valid_email(&field.value) {
                Html(r#"<span class="error">Invalid email</span>"#.to_string())
            } else if email_exists(&field.value).await {
                Html(r#"<span class="error">Email already taken</span>"#.to_string())
            } else {
                Html(r#"<span class="success">✓</span>"#.to_string())
            }
        }
        _ => Html(String::new())
    }
}
```

```html
<form hx-post="/users">
    <input
        name="email"
        hx-post="/validate/field"
        hx-trigger="blur"
        hx-target="next .validation-message" />
    <span class="validation-message"></span>

    <button type="submit">Submit</button>
</form>
```

### Pattern 5: Modal Dialogs

```rust
async fn show_delete_modal(Path(id): Path<i64>) -> Html<String> {
    let user = get_user(id).await;
    Html(format!(r#"
        <div class="modal" id="delete-modal">
            <div class="modal-content">
                <h3>Delete User</h3>
                <p>Are you sure you want to delete {}?</p>
                <button
                    hx-delete="/users/{id}"
                    hx-target="#user-{id}"
                    hx-swap="outerHTML">
                    Confirm Delete
                </button>
                <button onclick="closeModal()">Cancel</button>
            </div>
        </div>
    "#, user.name))
}
```

```html
<button
    hx-get="/users/{{id}}/delete-modal"
    hx-target="body"
    hx-swap="beforeend">
    Delete
</button>
```

## Advanced Techniques

### 1. Optimistic UI Updates

```html
<button
    hx-post="/todos/{{id}}/complete"
    hx-swap="outerHTML"
    hx-target="closest .todo-item"
    class="todo-item">

    <span class="htmx-request">✓ Completed</span>
    <span class="htmx-settling">{{ title }}</span>
</button>
```

### 2. Out-of-Band Swaps (Multiple Updates)

```rust
async fn create_todo(Form(data): Form<CreateTodo>) -> Html<String> {
    let todo = insert_todo(data).await;
    let count = count_todos().await;

    Html(format!(r#"
        <!-- Main todo list -->
        <div class="todo-item" id="todo-{}">{}</div>

        <!-- Update counter (out-of-band) -->
        <div id="todo-count" hx-swap-oob="true">
            {} todos
        </div>
    "#, todo.id, todo.title, count))
}
```

### 3. Polling for Updates

```html
<div
    hx-get="/notifications"
    hx-trigger="every 30s"
    hx-target="#notifications">
    <!-- Notifications will update every 30 seconds -->
</div>
```

### 4. WebSocket-like Behavior with SSE

```rust
use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};

async fn events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = stream::repeat_with(|| {
        Event::default()
            .event("message")
            .data(r#"<div hx-swap-oob="true" id="time">{}</div>"#)
    })
    .throttle(Duration::from_secs(1));

    Sse::new(stream)
}
```

```html
<div hx-ext="sse" sse-connect="/events" sse-swap="message">
    <div id="time">Connecting...</div>
</div>
```

## Integration with RustForge Features

### With rf-validation

```rust
use rf_validation::Validate;

async fn create_user(Form(data): Form<CreateUserForm>) -> Response {
    match data.validate() {
        Ok(_) => {
            let user = insert_user(data).await;
            (StatusCode::OK, Html(user_row_template(&user))).into_response()
        }
        Err(errors) => {
            (StatusCode::UNPROCESSABLE_ENTITY, Html(format!(r#"
                <div class="errors">
                    {}
                </div>
            "#, errors.to_html()))).into_response()
        }
    }
}
```

### With rf-auth

```rust
use rf_auth::Auth;

async fn profile_edit(
    Auth(user): Auth<User>
) -> Html<String> {
    Html(format!(r#"
        <form hx-post="/profile" hx-target="#profile">
            <input name="name" value="{}" />
            <button type="submit">Update</button>
        </form>
    "#, user.name))
}
```

### With rf-cache

```rust
use rf_cache::Cache;

async fn cached_widget(
    Extension(cache): Extension<Cache>
) -> Html<String> {
    cache.remember("widget:daily", Duration::from_secs(86400), || async {
        generate_expensive_widget().await
    }).await
}
```

## htmx Extensions

### Popular Extensions:

1. **Loading States**
```html
<div hx-ext="loading-states">
    <button hx-post="/submit"
            data-loading="Submitting..."
            data-loading-class="loading">
        Submit
    </button>
</div>
```

2. **Class Tools**
```html
<div hx-ext="class-tools">
    <div
        hx-post="/toggle"
        hx-target="this"
        classes="add htmx-settling:fade-in">
        Click me
    </div>
</div>
```

3. **Response Targets**
```html
<form hx-ext="response-targets" hx-post="/submit">
    <div hx-target-error="#error-div"></div>
    <div id="error-div"></div>
</form>
```

## Performance Optimization

### 1. Use Request Indicators

```html
<style>
    .htmx-request .spinner { display: inline-block; }
    .spinner { display: none; }
</style>

<button hx-post="/action">
    <span>Click me</span>
    <span class="spinner">Loading...</span>
</button>
```

### 2. Debounce User Input

```html
<input
    hx-get="/search"
    hx-trigger="keyup changed delay:500ms" />
```

### 3. Use Lazy Loading

```html
<div
    hx-get="/expensive-component"
    hx-trigger="load"
    hx-indicator=".spinner">
    <span class="spinner">Loading...</span>
</div>
```

## Testing htmx Endpoints

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_htmx_endpoint() {
        let app = Router::new()
            .route("/users/:id", get(users_show));

        let server = TestServer::new(app).unwrap();

        let response = server.get("/users/1")
            .add_header("HX-Request", "true")
            .await;

        assert_eq!(response.status_code(), 200);
        assert!(response.text().contains("user-1"));
    }
}
```

## Migration from Livewire

| Livewire Concept | htmx Equivalent |
|------------------|-----------------|
| `wire:model` | `hx-post` + `hx-trigger="input"` |
| `wire:click` | `hx-post` + `hx-trigger="click"` |
| `wire:submit` | `hx-post` on form |
| `wire:loading` | `htmx-indicator` class |
| `wire:poll` | `hx-trigger="every 2s"` |
| `wire:ignore` | `hx-preserve` |
| `$emit()` | htmx events or OOB swaps |
| `$refresh` | `hx-get` with same URL |

## Best Practices

1. **Keep Handlers Focused**: Each endpoint should return a focused HTML fragment
2. **Use IDs for Targets**: Makes swapping predictable
3. **Progressive Enhancement**: Ensure forms work without JS
4. **Cache Aggressively**: Use `rf-cache` for expensive renders
5. **Handle Errors Gracefully**: Return appropriate HTTP status codes
6. **Security**: Always validate and sanitize on the server
7. **Use CSRF Protection**: Include CSRF tokens in forms

## Complete Example: Todo App

```rust
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, Form},
    response::Html,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: i64,
    title: String,
    completed: bool,
}

async fn index() -> Html<&'static str> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <script src="https://unpkg.com/htmx.org@1.9.10"></script>
        <style>
            .completed { text-decoration: line-through; opacity: 0.6; }
            .htmx-indicator { display: none; }
            .htmx-request .htmx-indicator { display: inline; }
        </style>
    </head>
    <body>
        <h1>htmx Todo App</h1>

        <form hx-post="/todos" hx-target="#todo-list" hx-swap="afterbegin">
            <input name="title" placeholder="New todo..." required />
            <button type="submit">Add</button>
        </form>

        <div id="todo-list" hx-get="/todos" hx-trigger="load"></div>
    </body>
    </html>
    "#)
}

async fn list_todos() -> Html<String> {
    let todos = get_all_todos().await;
    let html = todos.iter()
        .map(|todo| todo_item_html(todo))
        .collect::<Vec<_>>()
        .join("\n");
    Html(html)
}

#[derive(Deserialize)]
struct CreateTodo {
    title: String,
}

async fn create_todo(Form(data): Form<CreateTodo>) -> Html<String> {
    let todo = insert_todo(data.title).await;
    Html(todo_item_html(&todo))
}

async fn toggle_todo(Path(id): Path<i64>) -> Html<String> {
    let todo = toggle_todo_status(id).await;
    Html(todo_item_html(&todo))
}

async fn delete_todo(Path(id): Path<i64>) -> StatusCode {
    delete_todo_by_id(id).await;
    StatusCode::OK
}

fn todo_item_html(todo: &Todo) -> String {
    format!(r#"
        <div id="todo-{}" class="{}">
            <input
                type="checkbox"
                {}
                hx-post="/todos/{}/toggle"
                hx-target="#todo-{}"
                hx-swap="outerHTML" />
            <span>{}</span>
            <button
                hx-delete="/todos/{}"
                hx-target="#todo-{}"
                hx-swap="outerHTML swap:1s"
                hx-confirm="Delete this todo?">
                ×
            </button>
        </div>
    "#,
        todo.id,
        if todo.completed { "completed" } else { "" },
        if todo.completed { "checked" } else { "" },
        todo.id,
        todo.id,
        todo.title,
        todo.id,
        todo.id
    )
}

pub fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/todos", get(list_todos))
        .route("/todos", post(create_todo))
        .route("/todos/:id/toggle", post(toggle_todo))
        .route("/todos/:id", delete(delete_todo))
}
```

## Conclusion

htmx provides a simpler, more performant alternative to Livewire for Rust applications. Its stateless architecture aligns perfectly with Rust's design principles, making it the recommended choice for building reactive interfaces in RustForge.

### Key Takeaways:

- ✅ **Simpler than Livewire** - No WebSocket complexity
- ✅ **Better for Rust** - Stateless, fast, memory-efficient
- ✅ **Progressive Enhancement** - Works without JavaScript
- ✅ **Excellent DX** - Minimal learning curve
- ✅ **Production Ready** - Used by companies worldwide

For more examples and patterns, see the [htmx documentation](https://htmx.org/docs/) and the [RustForge examples](../../examples/) directory.
