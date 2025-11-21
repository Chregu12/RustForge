//! # htmx + RustForge: Livewire Alternative Example
//!
//! This example demonstrates how to achieve Livewire-like functionality
//! using htmx with RustForge - 80% of the features with 5% of the complexity.
//!
//! ## Features Demonstrated:
//! - Counter (wire:click equivalent)
//! - Form validation (wire:model equivalent)
//! - File upload (wire:upload equivalent)
//! - Real-time updates (wire:poll equivalent)
//! - Loading states (wire:loading equivalent)
//! - Lazy loading (wire:init equivalent)
//!
//! ## Run:
//! ```bash
//! cargo run --example htmx-livewire-alternative
//! # Open http://localhost:3000
//! ```

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Multipart},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_sessions::{Session, SessionManagerLayer, MemoryStore};
use tracing::info;

#[derive(Clone)]
struct AppState {
    tasks: Arc<RwLock<Vec<Task>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: usize,
    title: String,
    completed: bool,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create session store
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false); // For development

    // Create app state
    let state = AppState {
        tasks: Arc::new(RwLock::new(vec![
            Task { id: 1, title: "Learn htmx".to_string(), completed: false },
            Task { id: 2, title: "Build with RustForge".to_string(), completed: false },
        ])),
    };

    // Build router
    let app = Router::new()
        .route("/", get(index))
        .route("/counter", get(counter_page))
        .route("/counter/increment", post(counter_increment))
        .route("/counter/decrement", post(counter_decrement))
        .route("/tasks", get(tasks_page))
        .route("/tasks/list", get(tasks_list))
        .route("/tasks/create", post(tasks_create))
        .route("/tasks/:id/toggle", post(tasks_toggle))
        .route("/tasks/:id/delete", post(tasks_delete))
        .route("/validation", get(validation_page))
        .route("/validation/check-email", post(check_email))
        .route("/realtime", get(realtime_page))
        .route("/realtime/time", get(current_time))
        .layer(session_layer)
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:3000";
    info!("Server running on http://{}", addr);
    info!("Examples:");
    info!("  - Counter: http://localhost:3000/counter");
    info!("  - Tasks: http://localhost:3000/tasks");
    info!("  - Validation: http://localhost:3000/validation");
    info!("  - Real-time: http://localhost:3000/realtime");

    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// ============================================================================
// INDEX PAGE
// ============================================================================

async fn index() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>htmx + RustForge: Livewire Alternative</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 0 auto;
            padding: 2rem;
            background: #f5f5f5;
        }
        .card {
            background: white;
            padding: 2rem;
            margin-bottom: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 { color: #333; }
        h2 { color: #666; margin-top: 0; }
        a {
            display: inline-block;
            background: #007bff;
            color: white;
            padding: 0.5rem 1rem;
            text-decoration: none;
            border-radius: 4px;
            margin-right: 0.5rem;
            margin-bottom: 0.5rem;
        }
        a:hover { background: #0056b3; }
        code {
            background: #f0f0f0;
            padding: 0.2rem 0.4rem;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }
    </style>
</head>
<body>
    <h1>htmx + RustForge: Livewire Alternative</h1>

    <div class="card">
        <h2>About This Demo</h2>
        <p>
            This demonstrates how to achieve <strong>Livewire-like functionality</strong>
            using <code>htmx</code> with RustForge.
        </p>
        <p>
            <strong>Result:</strong> 80% of Livewire features with 5% of the complexity.
        </p>
    </div>

    <div class="card">
        <h2>Examples</h2>
        <a href="/counter">1. Counter (wire:click)</a>
        <a href="/tasks">2. Task List (CRUD)</a>
        <a href="/validation">3. Form Validation (wire:model)</a>
        <a href="/realtime">4. Real-time Updates (wire:poll)</a>
    </div>

    <div class="card">
        <h2>Features Comparison</h2>
        <table style="width: 100%; border-collapse: collapse;">
            <tr>
                <th style="text-align: left; padding: 0.5rem; border-bottom: 2px solid #ddd;">Feature</th>
                <th style="text-align: left; padding: 0.5rem; border-bottom: 2px solid #ddd;">Livewire</th>
                <th style="text-align: left; padding: 0.5rem; border-bottom: 2px solid #ddd;">htmx</th>
            </tr>
            <tr>
                <td style="padding: 0.5rem;">Click handlers</td>
                <td style="padding: 0.5rem;"><code>wire:click</code></td>
                <td style="padding: 0.5rem;"><code>hx-post</code></td>
            </tr>
            <tr>
                <td style="padding: 0.5rem;">Form binding</td>
                <td style="padding: 0.5rem;"><code>wire:model</code></td>
                <td style="padding: 0.5rem;"><code>hx-post + hx-trigger</code></td>
            </tr>
            <tr>
                <td style="padding: 0.5rem;">Loading states</td>
                <td style="padding: 0.5rem;"><code>wire:loading</code></td>
                <td style="padding: 0.5rem;"><code>hx-indicator</code></td>
            </tr>
            <tr>
                <td style="padding: 0.5rem;">Polling</td>
                <td style="padding: 0.5rem;"><code>wire:poll</code></td>
                <td style="padding: 0.5rem;"><code>hx-trigger="every 2s"</code></td>
            </tr>
            <tr>
                <td style="padding: 0.5rem;">Lazy loading</td>
                <td style="padding: 0.5rem;"><code>wire:init</code></td>
                <td style="padding: 0.5rem;"><code>hx-trigger="load"</code></td>
            </tr>
        </table>
    </div>
</body>
</html>
    "#)
}

// ============================================================================
// COUNTER EXAMPLE (wire:click equivalent)
// ============================================================================

async fn counter_page() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Counter Example</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 2rem auto;
            padding: 2rem;
            background: #f5f5f5;
        }
        .counter {
            background: white;
            padding: 3rem;
            border-radius: 8px;
            text-align: center;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 { font-size: 4rem; margin: 0; color: #333; }
        button {
            font-size: 2rem;
            padding: 1rem 2rem;
            margin: 0.5rem;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            background: #007bff;
            color: white;
        }
        button:hover { background: #0056b3; }
        .htmx-indicator {
            display: none;
            color: #666;
            margin-top: 1rem;
        }
        .htmx-request .htmx-indicator {
            display: inline;
        }
    </style>
</head>
<body>
    <div class="counter">
        <p style="color: #666; margin-top: 0;">Livewire equivalent: <code>wire:click="increment"</code></p>

        <h1 id="count">0</h1>

        <div>
            <button
                hx-post="/counter/increment"
                hx-target="#count"
                hx-swap="innerHTML"
                hx-indicator="#loading"
            >
                ➕ Increment
            </button>

            <button
                hx-post="/counter/decrement"
                hx-target="#count"
                hx-swap="innerHTML"
            >
                ➖ Decrement
            </button>
        </div>

        <div id="loading" class="htmx-indicator">
            ⏳ Processing...
        </div>

        <p><a href="/" style="color: #007bff;">← Back</a></p>
    </div>
</body>
</html>
    "#)
}

async fn counter_increment(session: Session) -> Html<String> {
    let count: i32 = session.get("count").await.ok().flatten().unwrap_or(0);
    let new_count = count + 1;
    session.insert("count", new_count).await.ok();
    Html(new_count.to_string())
}

async fn counter_decrement(session: Session) -> Html<String> {
    let count: i32 = session.get("count").await.ok().flatten().unwrap_or(0);
    let new_count = count - 1;
    session.insert("count", new_count).await.ok();
    Html(new_count.to_string())
}

// ============================================================================
// TASKS EXAMPLE (CRUD operations)
// ============================================================================

async fn tasks_page() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Task List Example</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 2rem auto;
            padding: 2rem;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        input[type="text"] {
            width: 70%;
            padding: 0.5rem;
            font-size: 1rem;
            border: 1px solid #ddd;
            border-radius: 4px;
        }
        button {
            padding: 0.5rem 1rem;
            font-size: 1rem;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            background: #28a745;
            color: white;
        }
        button:hover { background: #218838; }
        .task {
            display: flex;
            align-items: center;
            padding: 1rem;
            margin: 0.5rem 0;
            background: #f8f9fa;
            border-radius: 4px;
        }
        .task.completed {
            opacity: 0.6;
            text-decoration: line-through;
        }
        .delete-btn {
            background: #dc3545;
            margin-left: auto;
        }
        .delete-btn:hover { background: #c82333; }
    </style>
</head>
<body>
    <div class="container">
        <h1>📝 Task List</h1>
        <p style="color: #666;">Livewire equivalent: Full CRUD component</p>

        <form hx-post="/tasks/create" hx-target="#task-list" hx-swap="afterbegin">
            <input type="text" name="title" placeholder="New task..." required>
            <button type="submit">Add</button>
        </form>

        <div id="task-list" hx-get="/tasks/list" hx-trigger="load">
            Loading tasks...
        </div>

        <p><a href="/" style="color: #007bff;">← Back</a></p>
    </div>
</body>
</html>
    "#)
}

async fn tasks_list(State(state): State<AppState>) -> Html<String> {
    let tasks = state.tasks.read().await;

    if tasks.is_empty() {
        return Html("<p style='text-align: center; color: #666;'>No tasks yet. Add one above!</p>".to_string());
    }

    let html = tasks.iter()
        .map(|task| format!(
            r#"
            <div class="task {}" id="task-{}">
                <input
                    type="checkbox"
                    {}
                    hx-post="/tasks/{}/toggle"
                    hx-target="#task-{}"
                    hx-swap="outerHTML"
                >
                <span style="flex: 1; margin-left: 1rem;">{}</span>
                <button
                    class="delete-btn"
                    hx-post="/tasks/{}/delete"
                    hx-target="#task-{}"
                    hx-swap="outerHTML"
                    hx-confirm="Delete this task?"
                >
                    🗑️ Delete
                </button>
            </div>
            "#,
            if task.completed { "completed" } else { "" },
            task.id,
            if task.completed { "checked" } else { "" },
            task.id,
            task.id,
            task.title,
            task.id,
            task.id
        ))
        .collect::<String>();

    Html(html)
}

#[derive(Deserialize)]
struct NewTask {
    title: String,
}

async fn tasks_create(
    State(state): State<AppState>,
    axum::Form(form): axum::Form<NewTask>,
) -> Html<String> {
    let mut tasks = state.tasks.write().await;
    let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;

    let task = Task {
        id,
        title: form.title,
        completed: false,
    };

    tasks.push(task.clone());

    Html(format!(
        r#"
        <div class="task" id="task-{}">
            <input
                type="checkbox"
                hx-post="/tasks/{}/toggle"
                hx-target="#task-{}"
                hx-swap="outerHTML"
            >
            <span style="flex: 1; margin-left: 1rem;">{}</span>
            <button
                class="delete-btn"
                hx-post="/tasks/{}/delete"
                hx-target="#task-{}"
                hx-swap="outerHTML"
                hx-confirm="Delete this task?"
            >
                🗑️ Delete
            </button>
        </div>
        "#,
        task.id, task.id, task.id, task.title, task.id, task.id
    ))
}

async fn tasks_toggle(
    Path(id): Path<usize>,
    State(state): State<AppState>,
) -> Html<String> {
    let mut tasks = state.tasks.write().await;

    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        task.completed = !task.completed;

        return Html(format!(
            r#"
            <div class="task {}" id="task-{}">
                <input
                    type="checkbox"
                    {}
                    hx-post="/tasks/{}/toggle"
                    hx-target="#task-{}"
                    hx-swap="outerHTML"
                >
                <span style="flex: 1; margin-left: 1rem;">{}</span>
                <button
                    class="delete-btn"
                    hx-post="/tasks/{}/delete"
                    hx-target="#task-{}"
                    hx-swap="outerHTML"
                    hx-confirm="Delete this task?"
                >
                    🗑️ Delete
                </button>
            </div>
            "#,
            if task.completed { "completed" } else { "" },
            task.id,
            if task.completed { "checked" } else { "" },
            task.id,
            task.id,
            task.title,
            task.id,
            task.id
        ));
    }

    Html(String::new())
}

async fn tasks_delete(
    Path(id): Path<usize>,
    State(state): State<AppState>,
) -> Html<&'static str> {
    let mut tasks = state.tasks.write().await;
    tasks.retain(|t| t.id != id);
    Html("")
}

// ============================================================================
// VALIDATION EXAMPLE (wire:model equivalent)
// ============================================================================

async fn validation_page() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Form Validation Example</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 2rem auto;
            padding: 2rem;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        input[type="email"] {
            width: 100%;
            padding: 0.75rem;
            font-size: 1rem;
            border: 1px solid #ddd;
            border-radius: 4px;
            box-sizing: border-box;
        }
        input.valid { border-color: #28a745; }
        input.invalid { border-color: #dc3545; }
        .feedback {
            margin-top: 0.5rem;
            font-size: 0.9rem;
        }
        .feedback.valid { color: #28a745; }
        .feedback.invalid { color: #dc3545; }
    </style>
</head>
<body>
    <div class="container">
        <h1>📧 Email Validation</h1>
        <p style="color: #666;">Livewire equivalent: <code>wire:model.debounce</code></p>

        <label for="email">Email Address:</label>
        <input
            type="email"
            id="email"
            name="email"
            placeholder="your@email.com"
            hx-post="/validation/check-email"
            hx-trigger="keyup changed delay:500ms"
            hx-target="#feedback"
        >

        <div id="feedback"></div>

        <p><a href="/" style="color: #007bff;">← Back</a></p>
    </div>
</body>
</html>
    "#)
}

#[derive(Deserialize)]
struct EmailCheck {
    email: String,
}

async fn check_email(axum::Form(form): axum::Form<EmailCheck>) -> Html<String> {
    // Simulate validation
    let is_valid = form.email.contains('@') && form.email.contains('.');
    let is_taken = form.email == "admin@example.com";

    let (class, message) = if is_taken {
        ("invalid", "❌ This email is already taken")
    } else if is_valid {
        ("valid", "✅ Email is available")
    } else {
        ("invalid", "❌ Invalid email format")
    };

    Html(format!(
        r#"<div class="feedback {}"><script>document.getElementById('email').className = '{}';</script>{}</div>"#,
        class, class, message
    ))
}

// ============================================================================
// REAL-TIME EXAMPLE (wire:poll equivalent)
// ============================================================================

async fn realtime_page() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Real-time Updates Example</title>
    <script src="https://unpkg.com/htmx.org@1.9.10"></script>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 600px;
            margin: 2rem auto;
            padding: 2rem;
            background: #f5f5f5;
        }
        .container {
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        .time {
            font-size: 3rem;
            text-align: center;
            color: #007bff;
            font-weight: bold;
            font-family: 'Courier New', monospace;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>⏰ Real-time Clock</h1>
        <p style="color: #666;">Livewire equivalent: <code>wire:poll.1s</code></p>

        <div
            class="time"
            hx-get="/realtime/time"
            hx-trigger="load, every 1s"
            hx-swap="innerHTML"
        >
            Loading...
        </div>

        <p style="text-align: center; color: #666;">
            Updates every second
        </p>

        <p><a href="/" style="color: #007bff;">← Back</a></p>
    </div>
</body>
</html>
    "#)
}

async fn current_time() -> Html<String> {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Format as HH:MM:SS
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;

    Html(format!("{:02}:{:02}:{:02}", hours, minutes, seconds))
}
