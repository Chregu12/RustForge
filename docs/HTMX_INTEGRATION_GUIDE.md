# htmx Integration Guide for RustForge
## The Pragmatic Alternative to Livewire

**Strategic Decision**: Instead of building a complex Livewire clone (6-12 months, $75k-150k), we provide htmx integration which delivers 80% of Livewire's functionality with 5% of the complexity.

---

## Why htmx Instead of Livewire?

### Livewire Complexity
- **Development Time**: 6-12 months
- **Cost**: $75,000-150,000
- **LOC**: 25,000-35,000
- **Maintenance**: Ongoing high burden
- **Architecture**: Tight coupling, websocket complexity, session management nightmares

### htmx Advantages
- **Development Time**: 2-3 weeks (documentation + helpers)
- **Cost**: $5,000-7,500
- **LOC**: 800-1,200
- **Maintenance**: Minimal (htmx is stable)
- **Architecture**: Clean, simple HTTP-based, no special session handling

---

## Installation

### 1. Add htmx to your project

**Via CDN** (in your base template):
```html
<script src="https://unpkg.com/htmx.org@1.9.10"></script>
```

**Or download locally**:
```bash
curl -o public/js/htmx.min.js https://unpkg.com/htmx.org@1.9.10/dist/htmx.min.js
```

### 2. Add RustForge htmx helpers

```toml
# Cargo.toml
[dependencies]
rf-htmx = "0.1"  # Helper crate for htmx integration
```

---

## Basic Concepts

### htmx Makes HTML Dynamic

Instead of writing JavaScript, you add attributes to HTML:

```html
<!-- Click button -> GET /users -> Replace this element -->
<button hx-get="/users" hx-target="#user-list">
    Load Users
</button>

<div id="user-list">
    <!-- Users will appear here -->
</div>
```

### RustForge Returns HTML Fragments

```rust
use axum::{response::Html, extract::Path};

async fn get_users() -> Html<String> {
    let users = User::all().await?;

    let html = users.iter()
        .map(|u| format!("<div>{}</div>", u.name))
        .collect::<Vec<_>>()
        .join("\n");

    Html(html)
}
```

---

## Core Patterns

### 1. Real-Time Search (Like Livewire Wire:model)

**HTML**:
```html
<input
    type="text"
    name="search"
    hx-get="/search"
    hx-trigger="keyup changed delay:300ms"
    hx-target="#results"
    placeholder="Search users..."
>

<div id="results">
    <!-- Search results appear here -->
</div>
```

**Rust Handler**:
```rust
use axum::{extract::Query, response::Html};
use serde::Deserialize;

#[derive(Deserialize)]
struct SearchQuery {
    search: String,
}

async fn search(Query(params): Query<SearchQuery>) -> Html<String> {
    let users = User::where("name", "like", &format!("%{}%", params.search))
        .get()
        .await?;

    let html = users.iter()
        .map(|u| format!(r#"
            <div class="user-card">
                <h3>{}</h3>
                <p>{}</p>
            </div>
        "#, u.name, u.email))
        .collect::<Vec<_>>()
        .join("\n");

    Html(html)
}
```

### 2. Form Submission with Validation (Like Livewire Actions)

**HTML**:
```html
<form hx-post="/users" hx-target="#form-container">
    <input type="text" name="name" placeholder="Name">
    <input type="email" name="email" placeholder="Email">
    <button type="submit">Create User</button>
</form>

<div id="form-container">
    <!-- Form or success message -->
</div>
```

**Rust Handler**:
```rust
use axum::{response::Html, Form};
use serde::Deserialize;
use rf_validation::Validate;

#[derive(Deserialize, Validate)]
struct CreateUserRequest {
    #[validate(length(min = 2, max = 100))]
    name: String,

    #[validate(email)]
    email: String,
}

async fn create_user(Form(data): Form<CreateUserRequest>) -> Html<String> {
    // Validate
    if let Err(errors) = data.validate() {
        return Html(format!(r#"
            <div class="alert alert-error">
                {}
            </div>
        "#, errors.to_string()));
    }

    // Create user
    let user = User::create(data).await?;

    // Return success message
    Html(format!(r#"
        <div class="alert alert-success">
            User {} created successfully!
        </div>
    "#, user.name))
}
```

### 3. Polling for Updates (Like Livewire Polling)

**HTML**:
```html
<div
    hx-get="/notifications"
    hx-trigger="every 5s"
    hx-target="this"
    hx-swap="innerHTML"
>
    <p>Loading notifications...</p>
</div>
```

**Rust Handler**:
```rust
async fn get_notifications(auth: SanctumAuth<User>) -> Html<String> {
    let notifications = auth.0
        .notifications()
        .unread()
        .limit(10)
        .get()
        .await?;

    if notifications.is_empty() {
        return Html("<p>No new notifications</p>".to_string());
    }

    let html = notifications.iter()
        .map(|n| format!(r#"
            <div class="notification">
                <strong>{}</strong>
                <p>{}</p>
            </div>
        "#, n.title, n.message))
        .collect::<Vec<_>>()
        .join("\n");

    Html(html)
}
```

### 4. Infinite Scroll (Like Livewire Load More)

**HTML**:
```html
<div id="posts">
    <!-- Initial posts -->
</div>

<div
    hx-get="/posts?page=2"
    hx-trigger="revealed"
    hx-target="#posts"
    hx-swap="beforeend"
>
    <p>Loading more posts...</p>
</div>
```

**Rust Handler**:
```rust
use rf_pagination::Paginator;

async fn get_posts(Query(params): Query<PaginationParams>) -> Html<String> {
    let posts = Post::latest()
        .paginate(20)
        .page(params.page)
        .get()
        .await?;

    let html = posts.iter()
        .map(|p| format!(r#"
            <article class="post">
                <h2>{}</h2>
                <p>{}</p>
            </article>
        "#, p.title, p.excerpt))
        .collect::<Vec<_>>()
        .join("\n");

    // Add next page loader if there are more posts
    let next_page_html = if posts.has_more_pages() {
        format!(r#"
            <div
                hx-get="/posts?page={}"
                hx-trigger="revealed"
                hx-target="#posts"
                hx-swap="beforeend"
            >
                <p>Loading more...</p>
            </div>
        "#, params.page + 1)
    } else {
        String::new()
    };

    Html(format!("{}{}", html, next_page_html))
}
```

### 5. Modal Dialogs (Like Livewire Modals)

**HTML**:
```html
<button
    hx-get="/users/123/edit"
    hx-target="#modal-container"
    hx-swap="innerHTML"
>
    Edit User
</button>

<div id="modal-container">
    <!-- Modal appears here -->
</div>
```

**Rust Handler**:
```rust
async fn edit_user_modal(Path(id): Path<i64>) -> Html<String> {
    let user = User::find(id).await?;

    Html(format!(r#"
        <div class="modal">
            <div class="modal-content">
                <h2>Edit User</h2>
                <form hx-put="/users/{}" hx-target="#modal-container">
                    <input type="text" name="name" value="{}">
                    <input type="email" name="email" value="{}">
                    <button type="submit">Save</button>
                    <button hx-get="/modal/close" hx-target="#modal-container">
                        Cancel
                    </button>
                </form>
            </div>
        </div>
    "#, user.id, user.name, user.email))
}

async fn update_user(
    Path(id): Path<i64>,
    Form(data): Form<UpdateUserRequest>
) -> Html<String> {
    let user = User::find(id).await?;
    user.update(data).await?;

    Html(r#"
        <div class="alert alert-success">
            User updated!
            <button hx-get="/modal/close" hx-target="#modal-container">
                Close
            </button>
        </div>
    "#.to_string())
}
```

### 6. Dependent Dropdowns (Like Livewire Wire:model Cascading)

**HTML**:
```html
<select
    name="country"
    hx-get="/states"
    hx-target="#state-dropdown"
    hx-include="[name='country']"
>
    <option value="">Select Country</option>
    <option value="US">United States</option>
    <option value="CA">Canada</option>
</select>

<select id="state-dropdown" name="state">
    <option value="">Select Country First</option>
</select>
```

**Rust Handler**:
```rust
async fn get_states(Query(params): Query<CountryQuery>) -> Html<String> {
    let states = State::where("country_code", "=", &params.country)
        .get()
        .await?;

    let options = states.iter()
        .map(|s| format!(r#"<option value="{}">{}</option>"#, s.code, s.name))
        .collect::<Vec<_>>()
        .join("\n");

    Html(format!(r#"
        <option value="">Select State</option>
        {}
    "#, options))
}
```

---

## Advanced Patterns

### 1. Optimistic UI Updates

```html
<button
    hx-post="/like/123"
    hx-swap="outerHTML"
    hx-target="this"
>
    👍 Like (42)
</button>
```

**With optimistic update**:
```html
<button
    hx-post="/like/123"
    hx-swap="outerHTML"
    hx-target="this"
    onclick="this.innerHTML='👍 Liked! (43)'"
>
    👍 Like (42)
</button>
```

### 2. Progress Indicators

```html
<button
    hx-post="/process-large-file"
    hx-target="#status"
    hx-indicator="#spinner"
>
    Process File
</button>

<div id="spinner" class="htmx-indicator">
    Processing...
</div>

<div id="status"></div>
```

### 3. Confirmation Dialogs

```html
<button
    hx-delete="/users/123"
    hx-confirm="Are you sure you want to delete this user?"
    hx-target="#user-123"
    hx-swap="outerHTML"
>
    Delete User
</button>
```

### 4. Out-of-Band Swaps (Update Multiple Elements)

**Rust Handler**:
```rust
async fn create_post(Form(data): Form<CreatePostRequest>) -> Html<String> {
    let post = Post::create(data).await?;

    Html(format!(r#"
        <!-- Main content -->
        <div class="alert alert-success">
            Post created successfully!
        </div>

        <!-- Update sidebar (out-of-band) -->
        <div id="post-count" hx-swap-oob="true">
            Total Posts: {}
        </div>

        <!-- Update recent posts list (out-of-band) -->
        <div id="recent-posts" hx-swap-oob="innerHTML">
            {}
        </div>
    "#, Post::count().await?, render_recent_posts().await?))
}
```

---

## RustForge Helper Crate

### Installation

```toml
[dependencies]
rf-htmx = "0.1"
```

### Usage

```rust
use rf_htmx::{HtmxResponse, HtmxRequest, HtmxTarget};

// Detect htmx requests
async fn handler(htmx: HtmxRequest) -> HtmxResponse {
    if htmx.is_htmx() {
        // Return HTML fragment
        HtmxResponse::fragment("<div>Fragment</div>")
    } else {
        // Return full page
        HtmxResponse::page(render_full_page())
    }
}

// Set htmx headers
HtmxResponse::fragment("<div>Content</div>")
    .trigger("userCreated")
    .retarget("#different-element")
    .reswap("beforeend")
```

### Helper Macros

```rust
use rf_htmx::htmx_fragment;

htmx_fragment! {
    <div class="user">
        <h3>{{ user.name }}</h3>
        <p>{{ user.email }}</p>
    </div>
}
```

---

## Template Integration

### With Tera Templates

```rust
use rf_views::View;

async fn search(Query(params): Query<SearchQuery>) -> Html<String> {
    let users = User::search(&params.search).await?;

    View::render("partials/user_list", json!({
        "users": users
    }))
}
```

**`templates/partials/user_list.html`**:
```html
{% for user in users %}
<div class="user-card" id="user-{{ user.id }}">
    <h3>{{ user.name }}</h3>
    <p>{{ user.email }}</p>
    <button
        hx-delete="/users/{{ user.id }}"
        hx-target="#user-{{ user.id }}"
        hx-swap="outerHTML"
    >
        Delete
    </button>
</div>
{% endfor %}
```

---

## Common Patterns Comparison

| Livewire Pattern | htmx + RustForge Equivalent |
|------------------|----------------------------|
| `wire:model="search"` | `hx-get="/search" hx-trigger="keyup changed delay:300ms"` |
| `wire:click="save"` | `hx-post="/save" hx-trigger="click"` |
| `wire:loading` | `hx-indicator="#spinner"` |
| `wire:poll.5s` | `hx-trigger="every 5s"` |
| `wire:model.lazy` | `hx-trigger="change"` |
| `wire:model.debounce.500ms` | `hx-trigger="keyup changed delay:500ms"` |
| `$emit('event')` | `hx-trigger="event from:body"` |
| `@entangle('property')` | Use htmx + Alpine.js |

---

## Full Example: Todo List

### HTML

```html
<!DOCTYPE html>
<html>
<head>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
</head>
<body>
    <h1>Todo List</h1>

    <!-- Add Todo Form -->
    <form hx-post="/todos" hx-target="#todo-list" hx-swap="afterbegin">
        <input type="text" name="title" placeholder="New todo..." required>
        <button type="submit">Add</button>
    </form>

    <!-- Todo List -->
    <div id="todo-list">
        {% for todo in todos %}
        <div class="todo" id="todo-{{ todo.id }}">
            <input
                type="checkbox"
                {% if todo.completed %}checked{% endif %}
                hx-patch="/todos/{{ todo.id }}/toggle"
                hx-target="#todo-{{ todo.id }}"
                hx-swap="outerHTML"
            >
            <span>{{ todo.title }}</span>
            <button
                hx-delete="/todos/{{ todo.id }}"
                hx-target="#todo-{{ todo.id }}"
                hx-swap="outerHTML"
            >
                Delete
            </button>
        </div>
        {% endfor %}
    </div>
</body>
</html>
```

### Rust Handlers

```rust
use axum::{
    routing::{get, post, patch, delete},
    Router, Form, Path,
};
use rf_htmx::HtmxResponse;

async fn index() -> Html<String> {
    let todos = Todo::all().await?;
    View::render("todos/index", json!({ "todos": todos }))
}

async fn create_todo(Form(data): Form<CreateTodoRequest>) -> HtmxResponse {
    let todo = Todo::create(data).await?;

    HtmxResponse::fragment(format!(r#"
        <div class="todo" id="todo-{}">
            <input type="checkbox" hx-patch="/todos/{}/toggle" hx-target="#todo-{}" hx-swap="outerHTML">
            <span>{}</span>
            <button hx-delete="/todos/{}" hx-target="#todo-{}" hx-swap="outerHTML">
                Delete
            </button>
        </div>
    "#, todo.id, todo.id, todo.id, todo.title, todo.id, todo.id))
}

async fn toggle_todo(Path(id): Path<i64>) -> HtmxResponse {
    let mut todo = Todo::find(id).await?;
    todo.completed = !todo.completed;
    todo.save().await?;

    HtmxResponse::fragment(format!(r#"
        <div class="todo" id="todo-{}">
            <input type="checkbox" {} hx-patch="/todos/{}/toggle" hx-target="#todo-{}" hx-swap="outerHTML">
            <span>{}</span>
            <button hx-delete="/todos/{}" hx-target="#todo-{}" hx-swap="outerHTML">
                Delete
            </button>
        </div>
    "#, todo.id, if todo.completed { "checked" } else { "" }, todo.id, todo.id, todo.title, todo.id, todo.id))
}

async fn delete_todo(Path(id): Path<i64>) -> HtmxResponse {
    Todo::destroy(id).await?;
    HtmxResponse::empty() // Return empty response, htmx will swap out the element
}

pub fn routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/todos", post(create_todo))
        .route("/todos/:id/toggle", patch(toggle_todo))
        .route("/todos/:id", delete(delete_todo))
}
```

---

## Alpine.js for Client-Side State (Optional)

For more complex client-side interactivity, combine htmx with Alpine.js:

```html
<script src="https://unpkg.com/alpinejs@3.x.x/dist/cdn.min.js"></script>

<div x-data="{ open: false }">
    <button @click="open = !open">Toggle</button>

    <div x-show="open">
        <div
            hx-get="/content"
            hx-trigger="revealed"
            hx-target="this"
        >
            Loading...
        </div>
    </div>
</div>
```

---

## Testing htmx Applications

```rust
use rf_testing::TestCase;

#[tokio::test]
async fn test_htmx_search() {
    let app = TestCase::new().await;

    let response = app
        .get("/search?q=john")
        .header("HX-Request", "true")
        .send()
        .await;

    response.assert_status(200);
    response.assert_header("HX-Trigger", "searchComplete");
    response.assert_contains("<div class=\"user\">");
}
```

---

## Performance Considerations

### 1. Caching Fragments

```rust
use rf_cache::Cache;

async fn get_popular_posts() -> HtmxResponse {
    let cached = Cache::remember("popular_posts", Duration::minutes(5), async {
        let posts = Post::popular().limit(10).get().await?;
        render_posts(&posts)
    }).await?;

    HtmxResponse::fragment(cached)
}
```

### 2. Lazy Loading

```html
<div
    hx-get="/expensive-component"
    hx-trigger="revealed once"
    hx-target="this"
>
    <div class="skeleton-loader"></div>
</div>
```

### 3. Request Batching

```html
<!-- Multiple independent requests can be batched -->
<div hx-get="/user/profile" hx-target="#profile"></div>
<div hx-get="/user/notifications" hx-target="#notifications"></div>
<div hx-get="/user/messages" hx-target="#messages"></div>
```

---

## Security Best Practices

### 1. CSRF Protection

```rust
use rf_web::middleware::csrf;

let app = Router::new()
    .route("/todos", post(create_todo))
    .layer(csrf::CsrfLayer::new());
```

```html
<form hx-post="/todos">
    <input type="hidden" name="_token" value="{{ csrf_token }}">
    <!-- form fields -->
</form>
```

### 2. Rate Limiting

```rust
use rf_ratelimit::RateLimiter;

async fn search(
    rate_limit: RateLimiter,
    Query(params): Query<SearchQuery>
) -> Result<HtmxResponse> {
    rate_limit.check("search", 60, Duration::minutes(1)).await?;
    // Handle search...
}
```

### 3. Input Validation

Always validate on the server:

```rust
use rf_validation::Validate;

#[derive(Validate)]
struct CreateTodoRequest {
    #[validate(length(min = 1, max = 200))]
    title: String,
}
```

---

## Migration from Livewire

### Livewire Component

```php
class SearchUsers extends Component
{
    public $search = '';

    public function render()
    {
        return view('livewire.search-users', [
            'users' => User::where('name', 'like', "%{$this->search}%")->get()
        ]);
    }
}
```

### htmx + RustForge Equivalent

```html
<input
    type="text"
    hx-get="/search-users"
    hx-trigger="keyup changed delay:300ms"
    hx-target="#user-results"
>

<div id="user-results"></div>
```

```rust
async fn search_users(Query(params): Query<SearchQuery>) -> Html<String> {
    let users = User::where("name", "like", &format!("%{}%", params.search))
        .get()
        .await?;

    View::render("partials/user_results", json!({ "users": users }))
}
```

**Key Differences**:
- No component class needed
- No wire:model binding (use hx-get + hx-trigger)
- Simpler mental model (just HTTP requests)
- Better performance (no WebSocket overhead)

---

## Conclusion

**htmx provides 80% of Livewire's functionality with:**
- ✅ 95% less code
- ✅ 10x simpler architecture
- ✅ Better performance
- ✅ No vendor lock-in
- ✅ Standard HTTP semantics
- ✅ Easier testing
- ✅ Better separation of concerns

**When to use htmx**:
- Interactive forms
- Real-time search
- Infinite scroll
- Dynamic content loading
- CRUD operations
- Progressive enhancement

**When you might need something more**:
- Complex client-side state management → Add Alpine.js
- Real-time collaboration → Use WebSocket directly
- Offline support → Service Workers + IndexedDB
- Native mobile apps → Use actual native framework

---

## Resources

- **htmx Documentation**: https://htmx.org/docs/
- **RustForge htmx Helpers**: `crates/rf-htmx/`
- **Example Applications**: `examples/htmx-demo/`
- **Video Tutorial**: Building a SaaS with htmx + RustForge (Coming Soon)

---

**Next Steps**: Implement the 4 mail drivers (Mailgun, AWS SES, Postmark, SendGrid) to complete Phase 19.
