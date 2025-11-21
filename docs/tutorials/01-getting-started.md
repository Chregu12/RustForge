# Getting Started with RustForge

**Estimated Time:** 30 minutes
**Prerequisites:** Rust 1.75+ installed, basic command line knowledge
**What You'll Learn:** Installing RustForge, creating your first project, understanding the structure, and running your first application

---

## Introduction

Welcome to RustForge! This tutorial will guide you through setting up RustForge and building your first web application. By the end of this tutorial, you'll have a working RustForge application responding to HTTP requests.

## Table of Contents

1. [Installation](#installation)
2. [Creating Your First Project](#creating-your-first-project)
3. [Understanding the Project Structure](#understanding-the-project-structure)
4. [Your First Route](#your-first-route)
5. [Working with Controllers](#working-with-controllers)
6. [Running the Application](#running-the-application)
7. [Next Steps](#next-steps)

---

## Installation

### Step 1: Install Rust

If you haven't already, install Rust using rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Verify your installation:

```bash
rustc --version
cargo --version
```

You should see version 1.75 or higher.

### Step 2: Install RustForge CLI

Install the RustForge command-line tool:

```bash
cargo install rustforge-cli
```

Verify the installation:

```bash
forge --version
```

You should see output like:
```
RustForge CLI v0.1.0
```

---

## Creating Your First Project

### Step 1: Generate a New Project

Use the `forge new` command to create a new project:

```bash
forge new hello-rustforge
cd hello-rustforge
```

This creates a new directory with a complete RustForge application structure.

### Step 2: Set Up Environment

Copy the example environment file:

```bash
cp .env.example .env
```

Open `.env` and configure your database connection (optional for this tutorial):

```env
DATABASE_URL=postgres://user:password@localhost/hello_rustforge
APP_ENV=local
APP_DEBUG=true
APP_KEY=base64:your-secret-key-here
```

For now, you can keep the defaults. We'll work with databases in the next tutorial.

---

## Understanding the Project Structure

Your new RustForge project has the following structure:

```
hello-rustforge/
├── src/
│   ├── main.rs              # Application entry point
│   ├── routes.rs            # Route definitions
│   ├── controllers/         # Request handlers
│   │   └── mod.rs
│   ├── models/              # Data models
│   │   └── mod.rs
│   └── views/               # Blade templates
│       └── welcome.blade.html
├── migrations/              # Database migrations
├── tests/                   # Test files
├── public/                  # Static assets
├── storage/                 # File storage
├── Cargo.toml              # Rust dependencies
└── .env                    # Environment configuration
```

**Key Files:**
- `src/main.rs` - Bootstraps the application and starts the web server
- `src/routes.rs` - Defines all HTTP routes
- `src/controllers/` - Contains your business logic
- `src/models/` - Database models and business entities
- `src/views/` - Blade template files

---

## Your First Route

Let's create a simple route that returns "Hello, RustForge!"

### Step 1: Open the Routes File

Open `src/routes.rs` in your favorite editor. You'll see:

```rust
use rf_routing::{Router, Route};
use crate::controllers;

pub fn register_routes() -> Router {
    Router::new()
        .route("/", Route::get(controllers::home::index))
}
```

### Step 2: Add a Simple Route

Let's add a new route that returns a plain text response:

```rust
use rf_routing::{Router, Route};
use rf_http::{Request, Response};

pub fn register_routes() -> Router {
    Router::new()
        .route("/", Route::get(welcome))
        .route("/hello", Route::get(hello))
}

async fn welcome(_req: Request) -> Response {
    Response::ok().body("Welcome to RustForge!")
}

async fn hello(_req: Request) -> Response {
    Response::ok().body("Hello, RustForge!")
}
```

### Step 3: Understanding the Code

- `Router::new()` - Creates a new router instance
- `.route("/hello", Route::get(hello))` - Registers a GET route at `/hello`
- `async fn hello(_req: Request) -> Response` - Async handler function
- `Response::ok().body(...)` - Creates a 200 OK response with text

---

## Working with Controllers

For better organization, let's move our logic to a controller.

### Step 1: Create a Welcome Controller

Create a new file `src/controllers/welcome_controller.rs`:

```rust
use rf_http::{Request, Response};

pub async fn index(_req: Request) -> Response {
    Response::ok().body("Welcome to RustForge!")
}

pub async fn hello(_req: Request) -> Response {
    Response::ok().body("Hello, RustForge!")
}

pub async fn greet(req: Request) -> Response {
    let name = req.param("name").unwrap_or("Guest");
    Response::ok().body(format!("Hello, {}!", name))
}
```

### Step 2: Register the Controller Module

Update `src/controllers/mod.rs`:

```rust
pub mod welcome_controller;
```

### Step 3: Update Routes

Update `src/routes.rs` to use the controller:

```rust
use rf_routing::{Router, Route};
use crate::controllers::welcome_controller;

pub fn register_routes() -> Router {
    Router::new()
        .route("/", Route::get(welcome_controller::index))
        .route("/hello", Route::get(welcome_controller::hello))
        .route("/greet/:name", Route::get(welcome_controller::greet))
}
```

### Step 4: Understanding Route Parameters

The route `/greet/:name` captures a URL parameter. For example:
- `/greet/Alice` → "Hello, Alice!"
- `/greet/Bob` → "Hello, Bob!"

---

## Running the Application

### Step 1: Build and Run

Start the development server:

```bash
forge serve
```

You should see:

```
   Compiling hello-rustforge v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 5.23s
     Running `target/debug/hello-rustforge`

🚀 RustForge server starting...
📍 Listening on http://127.0.0.1:8000
🔧 Environment: local
✅ Server ready to accept connections
```

### Step 2: Test Your Routes

Open your browser or use `curl` to test:

```bash
# Welcome page
curl http://127.0.0.1:8000/
# Output: Welcome to RustForge!

# Hello endpoint
curl http://127.0.0.1:8000/hello
# Output: Hello, RustForge!

# Greet with parameter
curl http://127.0.0.1:8000/greet/Alice
# Output: Hello, Alice!
```

### Step 3: Auto-Reload on Changes

To enable auto-reload during development, use the `--watch` flag:

```bash
forge serve --watch
```

Now when you save files, the server automatically reloads!

---

## Working with Templates

Let's create a proper HTML view using Blade templates.

### Step 1: Create a Template

Create `src/views/welcome.blade.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome to RustForge</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 800px;
            margin: 50px auto;
            padding: 20px;
            text-align: center;
        }
        h1 { color: #e74c3c; }
        .code {
            background: #f4f4f4;
            padding: 10px;
            border-radius: 5px;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <h1>Welcome to RustForge</h1>
    <p>You're running RustForge {{ version }}</p>

    @if(name)
        <p>Hello, <strong>{{ name }}</strong>!</p>
    @else
        <p>Hello, Guest!</p>
    @endif

    <div class="code">
        <p>Edit this template at: <code>src/views/welcome.blade.html</code></p>
    </div>
</body>
</html>
```

### Step 2: Update Controller to Use View

Update `src/controllers/welcome_controller.rs`:

```rust
use rf_http::{Request, Response};
use rf_views::View;
use serde_json::json;

pub async fn index(_req: Request) -> Response {
    View::make("welcome")
        .with("version", "1.0.0")
        .with("name", None::<String>)
        .render()
}

pub async fn greet(req: Request) -> Response {
    let name = req.param("name").unwrap_or("Guest");

    View::make("welcome")
        .with("version", "1.0.0")
        .with("name", name)
        .render()
}
```

### Step 3: Test the Views

Visit `http://127.0.0.1:8000/` and `http://127.0.0.1:8000/greet/Alice` in your browser to see the rendered HTML!

---

## Understanding Request Flow

Let's trace what happens when someone visits `http://127.0.0.1:8000/greet/Alice`:

1. **Router** receives the request and matches it to `/greet/:name`
2. **Route parameter** extraction: `name = "Alice"`
3. **Controller** handler `greet()` is called with the request
4. **View** is created with data: `{"version": "1.0.0", "name": "Alice"}`
5. **Blade engine** compiles and renders the template
6. **Response** is returned with HTML content
7. **Browser** displays the page

---

## Next Steps

Congratulations! You've created your first RustForge application. You learned how to:

- ✅ Install RustForge and create a new project
- ✅ Define routes and route parameters
- ✅ Create controllers to organize your code
- ✅ Use Blade templates for views
- ✅ Pass data to views
- ✅ Run the development server

### Continue Learning

Now that you understand the basics, try these tutorials:

1. **[Building a Blog](./02-building-a-blog/01-setup.md)** - Learn databases, models, and authentication (4-5 hours)
2. **[API Development](./03-api-development.md)** - Build RESTful APIs (2 hours)
3. **[Testing](./05-testing.md)** - Test your applications (2 hours)

### Explore the Documentation

- [Routing Guide](../guides/routing.md)
- [Controllers Guide](../guides/controllers.md)
- [Views & Blade](../guides/views.md)
- [Database & ORM](../guides/database.md)

### Join the Community

- GitHub: [github.com/rustforge/rustforge](https://github.com/rustforge/rustforge)
- Discord: [discord.gg/rustforge](https://discord.gg/rustforge)
- Documentation: [docs.rustforge.dev](https://docs.rustforge.dev)

---

## Troubleshooting

### Server Won't Start

**Error:** `Address already in use`

**Solution:** Another process is using port 8000. Either stop that process or use a different port:

```bash
forge serve --port 8080
```

### Compilation Errors

**Error:** `cannot find trait HttpResponse`

**Solution:** Make sure all dependencies are installed:

```bash
cargo clean
cargo build
```

### Template Not Found

**Error:** `Template 'welcome' not found`

**Solution:** Ensure your template is in `src/views/` with `.blade.html` extension:

```bash
ls src/views/welcome.blade.html
```

---

## Summary

In this tutorial, you:

1. Installed RustForge CLI
2. Created a new project
3. Defined routes and route parameters
4. Created a controller
5. Built Blade templates
6. Ran the development server

**Time to complete:** ~30 minutes ✅

Ready for more? Head to the [Building a Blog Tutorial](./02-building-a-blog/01-setup.md) to learn about databases, authentication, and CRUD operations!
