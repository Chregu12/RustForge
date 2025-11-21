# Phase 12 Integration Guide

Complete guide for integrating RustForge Phase 12 full-stack and CMS features into your applications.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Template Engine (rf-blade)](#template-engine)
3. [Asset Pipeline (rf-vite)](#asset-pipeline)
4. [Live Reload (rf-livereload)](#live-reload)
5. [Content Management (rf-cms)](#content-management)
6. [Complete Integration](#complete-integration)
7. [Best Practices](#best-practices)
8. [Common Patterns](#common-patterns)
9. [Troubleshooting](#troubleshooting)

---

## Quick Start

### Minimal Full-Stack Application

```rust
// Cargo.toml
[dependencies]
rf-blade = { path = "../../crates/rf-blade" }
rf-vite = { path = "../../crates/rf-vite" }
rf-livereload = { path = "../../crates/rf-livereload" }
axum = "0.7"
tokio = { version = "1.35", features = ["full"] }
serde_json = "1.0"

// src/main.rs
use std::sync::Arc;
use axum::{Router, routing::get, extract::State, response::Html};
use rf_blade::BladeEngine;
use serde_json::json;

#[derive(Clone)]
struct AppState {
    blade: Arc<BladeEngine>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize template engine
    let blade = Arc::new(BladeEngine::new("templates/")?);

    let state = AppState { blade };

    let app = Router::new()
        .route("/", get(home))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn home(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = state.blade.render("home", json!({
        "title": "Welcome to RustForge"
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}
```

```html
<!-- templates/home.blade.html -->
<!DOCTYPE html>
<html>
<head>
    <title>{{ $title }}</title>
</head>
<body>
    <h1>{{ $title }}</h1>
</body>
</html>
```

Run: `cargo run` → Open http://localhost:3000

---

## Template Engine

### 1. Basic Setup

```rust
use rf_blade::BladeEngine;

let blade = BladeEngine::new("templates/")?;
```

### 2. Template Inheritance

```html
<!-- templates/layouts/app.blade.html -->
<!DOCTYPE html>
<html>
<head>
    <title>@yield('title')</title>
</head>
<body>
    <header>
        @yield('header')
    </header>

    <main>
        @yield('content')
    </main>

    <footer>
        @yield('footer', 'Default Footer')
    </footer>
</body>
</html>
```

```html
<!-- templates/pages/about.blade.html -->
@extends('layouts.app')

@section('title', 'About Us')

@section('content')
    <h1>About Our Company</h1>
    <p>We build amazing things!</p>
@endsection
```

### 3. Directives

```html
<!-- Conditionals -->
@if($user.is_admin)
    <p>Admin Panel Access</p>
@elseif($user.is_member)
    <p>Member Area</p>
@else
    <p>Guest Access</p>
@endif

<!-- Loops -->
@foreach($posts as $post)
    <article>
        <h2>{{ $post.title }}</h2>
        <p>{{ $post.excerpt }}</p>
    </article>
@endforeach

<!-- Authentication -->
@auth
    <p>Welcome back, {{ $user.name }}!</p>
@endauth

@guest
    <a href="/login">Login</a>
@endguest
```

### 4. Components

```html
<!-- templates/components/card.blade.html -->
<div class="card">
    <div class="card-header">
        <h3>{{ $title }}</h3>
    </div>
    <div class="card-body">
        {{ $content }}
    </div>
</div>
```

Usage:
```html
<x-card title="User Info" content="Details go here" />
```

### 5. Custom Directives

```rust
blade.directive("datetime", |value| {
    // Format datetime
    format!("<time>{}</time>", value)
})?;

blade.directive("currency", |value| {
    // Format currency
    format!("${:.2}", value.parse::<f64>().unwrap_or(0.0))
})?;
```

Usage in templates:
```html
@datetime(2024-11-14)
@currency(99.99)
```

---

## Asset Pipeline

### 1. Setup Vite

```bash
npm init -y
npm install -D vite
```

```javascript
// vite.config.js
import { defineConfig } from 'vite';

export default defineConfig({
    build: {
        manifest: true,
        outDir: 'public/build',
        rollupOptions: {
            input: {
                app: 'resources/js/app.js',
                styles: 'resources/css/app.css',
            },
        },
    },
});
```

### 2. Development Mode

```rust
use rf_vite::ViteConfig;

let vite_dev = std::env::var("VITE_DEV")
    .unwrap_or_else(|_| "true".to_string()) == "true";

if vite_dev {
    let vite = ViteConfig::new(".")
        .entry("resources/js/app.js")
        .entry("resources/css/app.css")
        .port(5173);

    tokio::spawn(async move {
        if let Ok(server) = vite.dev_server().await {
            println!("Vite dev server started");
        }
    });
}
```

### 3. Template Integration

```html
@if($vite_dev)
    <!-- Development -->
    <script type="module" src="http://localhost:5173/@vite/client"></script>
    <script type="module" src="http://localhost:5173/resources/js/app.js"></script>
    <link rel="stylesheet" href="http://localhost:5173/resources/css/app.css">
@else
    <!-- Production -->
    <link rel="stylesheet" href="/build/assets/app.css">
    <script type="module" src="/build/assets/app.js"></script>
@endif
```

### 4. Production Build

```bash
# Build assets
npm run build

# Run app in production mode
VITE_DEV=false cargo run --release
```

### 5. Asset Helper

```rust
// Helper function for templates
fn asset_url(path: &str, dev: bool) -> String {
    if dev {
        format!("http://localhost:5173/{}", path)
    } else {
        // Load from manifest
        format!("/build/assets/{}", path)
    }
}
```

---

## Live Reload

### 1. Basic Setup

```rust
use rf_livereload::LiveReload;

let reload = LiveReload::new()
    .watch("templates")
    .watch("resources/css")
    .watch("resources/js")
    .debounce_ms(300);

let server = reload.start().await?;
println!("LiveReload on port {}", server.port());
```

### 2. Template Integration

```html
<head>
    <!-- Other head content -->

    @if($dev_mode)
        <!-- LiveReload Script -->
        <script>
        (function() {
            const ws = new WebSocket('ws://localhost:35729');
            ws.onmessage = (event) => {
                const data = JSON.parse(event.data);
                if (data.kind === 'CssOnly') {
                    // Reload CSS only
                    const links = document.querySelectorAll('link[rel="stylesheet"]');
                    links.forEach(link => {
                        const href = link.href.split('?')[0];
                        link.href = href + '?reload=' + Date.now();
                    });
                } else {
                    // Full page reload
                    window.location.reload();
                }
            };
            ws.onerror = () => console.log('LiveReload disconnected');
        })();
        </script>
    @endif
</head>
```

### 3. Custom Patterns

```rust
let reload = LiveReload::new()
    .watch("templates")
    .pattern("*.blade.html")  // Only Blade templates
    .pattern("*.css")         // Only CSS files
    .debounce_ms(500);        // Wait 500ms before reload
```

### 4. Manual Triggers

```rust
use rf_livereload::ReloadKind;

// Trigger reload programmatically
reload.trigger(ReloadKind::Full)?;        // Full reload
reload.trigger(ReloadKind::CssOnly)?;     // CSS only
reload.trigger(ReloadKind::JsModule)?;    // JS module
```

---

## Content Management

### 1. Media Library Setup

```rust
use rf_cms::MediaLibrary;

let media = Arc::new(MediaLibrary::new("storage/media"));
```

### 2. File Upload

```rust
use axum::extract::Multipart;

async fn upload_file(
    State(media): State<Arc<MediaLibrary>>,
    mut multipart: Multipart,
) -> Result<Json<FileResponse>, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(filename) = field.file_name() {
            let data = field.bytes().await.unwrap();

            // Upload file
            let file = media.upload(filename, data.to_vec())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(Json(FileResponse {
                id: file.id.clone(),
                url: format!("/storage/media/{}", file.filename),
                filename: file.filename,
                size: file.size,
            }));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}
```

### 3. Image Processing

```rust
// Generate thumbnail
let thumbnail = media.thumbnail(&file_id, 200, 200).await?;

// Get URL
let url = media.url(&file_id).await;

// Get file info
let info = media.get_file(&file_id).await?;
```

### 4. WYSIWYG Integration

```rust
use rf_cms::EditorConfig;

// TinyMCE
let tinymce = EditorConfig::tinymce()
    .plugins(&["image", "link", "lists", "code"])
    .toolbar("undo redo | formatselect | bold italic | image link");

// CKEditor
let ckeditor = EditorConfig::ckeditor()
    .toolbar_items(&["heading", "bold", "italic", "link", "imageUpload"])
    .image_upload_url("/api/upload");
```

Template:
```html
<textarea id="editor">{{ $content }}</textarea>

<script src="https://cdn.tiny.cloud/1/YOUR-KEY/tinymce/6/tinymce.min.js"></script>
<script>
tinymce.init({
    selector: '#editor',
    plugins: 'image link lists code',
    toolbar: 'undo redo | formatselect | bold italic | image link',
    file_picker_callback: (callback) => {
        // Open media library
        openMediaLibrary((file) => {
            callback(file.url, { alt: file.filename });
        });
    }
});
</script>
```

### 5. Content Sanitization

```rust
use rf_cms::ContentSanitizer;

let sanitizer = ContentSanitizer::new();

// Remove dangerous content
let safe_html = sanitizer.sanitize(user_input)?;

// Strip all tags
let plain_text = sanitizer.strip_tags(html)?;
```

### 6. Revisions

```rust
use rf_cms::RevisionManager;

let revisions = RevisionManager::new("storage/revisions");

// Save revision
revisions.save("post-123", "v2", &content).await?;

// List all revisions
let history = revisions.list("post-123").await?;

// Get specific revision
let old = revisions.get("post-123", "v1").await?;

// Rollback
let restored = revisions.rollback("post-123", "v1").await?;
```

---

## Complete Integration

### Full Application Example

```rust
use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Multipart},
    response::{Html, IntoResponse},
    http::StatusCode,
};
use rf_blade::BladeEngine;
use rf_vite::ViteConfig;
use rf_livereload::LiveReload;
use rf_cms::MediaLibrary;
use serde_json::json;

#[derive(Clone)]
struct AppState {
    blade: Arc<BladeEngine>,
    media: Arc<MediaLibrary>,
    vite_dev: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize all Phase 12 crates
    let blade = Arc::new(BladeEngine::new("templates/")?);
    let media = Arc::new(MediaLibrary::new("storage/media"));

    let vite_dev = std::env::var("VITE_DEV")
        .unwrap_or_else(|_| "true".to_string()) == "true";

    // Development tools
    if vite_dev {
        // LiveReload
        let reload = LiveReload::new()
            .watch("templates")
            .watch("resources");

        tokio::spawn(async move {
            reload.start().await.ok();
        });

        // Vite
        let vite = ViteConfig::new(".")
            .entry("resources/js/app.js");

        tokio::spawn(async move {
            vite.dev_server().await.ok();
        });
    }

    let state = AppState { blade, media, vite_dev };

    // Build routes
    let app = Router::new()
        .route("/", get(home))
        .route("/posts/:id", get(show_post))
        .route("/posts/create", get(create_form).post(create_post))
        .route("/media/upload", post(upload_media))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn home(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = state.blade.render("home", json!({
        "vite_dev": state.vite_dev
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

async fn show_post(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    // Fetch post from database
    let post = get_post(&id)?;

    let html = state.blade.render("posts.show", json!({
        "post": post,
        "vite_dev": state.vite_dev
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

async fn create_form(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = state.blade.render("posts.create", json!({
        "vite_dev": state.vite_dev
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

async fn create_post(
    State(state): State<AppState>,
    // Parse form data
) -> Result<impl IntoResponse, StatusCode> {
    // Create post logic
    Ok((StatusCode::SEE_OTHER, [("Location", "/")]))
}

async fn upload_media(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(filename) = field.file_name() {
            let data = field.bytes().await.unwrap();
            let file = state.media.upload(filename, data.to_vec()).await.unwrap();

            return Ok(Json(UploadResponse {
                url: format!("/storage/media/{}", file.filename)
            }));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}
```

---

## Best Practices

### 1. Template Organization

```
templates/
├── layouts/
│   ├── app.blade.html          # Main layout
│   ├── admin.blade.html        # Admin layout
│   └── email.blade.html        # Email layout
├── components/
│   ├── navigation.blade.html
│   ├── footer.blade.html
│   └── card.blade.html
├── pages/
│   ├── home.blade.html
│   └── about.blade.html
└── emails/
    └── welcome.blade.html
```

### 2. Separate Dev/Prod Configuration

```rust
#[derive(Clone)]
struct Config {
    vite_dev: bool,
    livereload: bool,
    asset_url: String,
}

impl Config {
    fn from_env() -> Self {
        let is_dev = cfg!(debug_assertions);

        Self {
            vite_dev: is_dev,
            livereload: is_dev,
            asset_url: if is_dev {
                "http://localhost:5173".to_string()
            } else {
                "/build".to_string()
            },
        }
    }
}
```

### 3. Error Handling

```rust
use rf_blade::BladeError;

async fn render_template(
    blade: &BladeEngine,
    template: &str,
    data: serde_json::Value,
) -> Result<String, AppError> {
    blade.render(template, data).await.map_err(|e| match e {
        BladeError::TemplateNotFound(name) => {
            AppError::NotFound(format!("Template {} not found", name))
        }
        BladeError::ParseError(msg) => {
            AppError::InternalError(format!("Template parse error: {}", msg))
        }
        _ => AppError::InternalError("Template rendering failed".to_string()),
    })
}
```

### 4. Asset Versioning

```rust
// Load manifest in production
use rf_vite::ViteManifest;

if !vite_dev {
    let manifest = ViteManifest::load(
        "public/build/manifest.json",
        PathBuf::from("public/build")
    ).await?;

    // Use manifest for versioned URLs
    let script_tag = manifest.script("resources/js/app.js")?;
}
```

### 5. Media Storage Strategy

```rust
// Local storage for development
let media = if cfg!(debug_assertions) {
    MediaLibrary::new("storage/media")
} else {
    // S3/cloud storage in production (when implemented)
    MediaLibrary::with_backend(S3Backend::new(config))
};
```

---

## Common Patterns

### Pattern 1: Reusable Layout with Dynamic Sections

```html
<!-- templates/layouts/app.blade.html -->
<!DOCTYPE html>
<html>
<head>
    <title>@yield('title', 'Default Title')</title>
    @yield('meta')
    @yield('styles')
</head>
<body>
    @yield('content')
    @yield('scripts')
</body>
</html>
```

### Pattern 2: Component with Slots

```html
<!-- templates/components/modal.blade.html -->
<div class="modal">
    <div class="modal-header">
        {{ $title }}
    </div>
    <div class="modal-body">
        {{ $body }}
    </div>
    <div class="modal-footer">
        {{ $footer }}
    </div>
</div>
```

### Pattern 3: Conditional Asset Loading

```rust
fn asset_tags(vite_dev: bool, entry: &str) -> String {
    if vite_dev {
        format!(
            r#"<script type="module" src="http://localhost:5173/@vite/client"></script>
<script type="module" src="http://localhost:5173/{}"></script>"#,
            entry
        )
    } else {
        // Load from manifest
        format!(r#"<script type="module" src="/build/{}"></script>"#, entry)
    }
}
```

### Pattern 4: Media Upload with Progress

```javascript
// Frontend upload with progress
async function uploadFile(file) {
    const formData = new FormData();
    formData.append('file', file);

    const response = await fetch('/media/upload', {
        method: 'POST',
        body: formData,
    });

    const result = await response.json();
    return result;
}
```

### Pattern 5: Content Versioning

```rust
async fn save_with_revision(
    content: &str,
    revisions: &RevisionManager,
    id: &str,
) -> Result<()> {
    // Get current version
    let current_version = revisions.get_latest(id).await?;

    // Save new revision
    let version = format!("v{}", current_version.version + 1);
    revisions.save(id, &version, content).await?;

    Ok(())
}
```

---

## Troubleshooting

### Issue: Templates Not Rendering

**Symptoms**: Blank page or error "Template not found"

**Solutions**:
```rust
// 1. Check template path
let blade = BladeEngine::new("templates/")?;  // ✅ Correct
let blade = BladeEngine::new("views/")?;      // ❌ Wrong path

// 2. Verify file extension
// ✅ templates/home.blade.html
// ✅ templates/home.html
// ❌ templates/home.tmpl

// 3. Check file exists
use std::path::Path;
assert!(Path::new("templates/home.blade.html").exists());
```

### Issue: Vite Dev Server Not Starting

**Symptoms**: Assets not loading in development

**Solutions**:
```bash
# 1. Install Vite
npm install -D vite

# 2. Check if already running
lsof -i :5173

# 3. Kill existing process
kill -9 $(lsof -t -i:5173)

# 4. Check vite.config.js exists
cat vite.config.js
```

### Issue: LiveReload Not Working

**Symptoms**: Page doesn't reload on file changes

**Solutions**:
```rust
// 1. Verify WebSocket connection in browser console
// Should see: WebSocket connection to 'ws://localhost:35729' established

// 2. Check file watching paths
let reload = LiveReload::new()
    .watch("templates")     // ✅ Correct
    .watch("views");        // ❌ Wrong path

// 3. Increase debounce if too sensitive
let reload = LiveReload::new()
    .debounce_ms(500);  // Wait 500ms before reload
```

### Issue: Media Upload Failing

**Symptoms**: 500 error on upload

**Solutions**:
```rust
// 1. Ensure directory exists
tokio::fs::create_dir_all("storage/media").await?;

// 2. Check permissions
// Unix: chmod 755 storage/media

// 3. Verify multipart parsing
while let Some(field) = multipart.next_field().await? {
    if let Some(filename) = field.file_name() {
        println!("Uploading: {}", filename);  // Debug log
        // ...
    }
}
```

### Issue: Asset 404 in Production

**Symptoms**: Assets not loading after build

**Solutions**:
```bash
# 1. Build assets first
npm run build

# 2. Verify manifest exists
ls -la public/build/manifest.json

# 3. Check manifest contents
cat public/build/manifest.json

# 4. Ensure correct paths in production
VITE_DEV=false cargo run
```

---

## Performance Tips

### 1. Template Caching

```rust
// Templates are automatically cached after first render
// Clear cache if needed:
blade.clear_cache().await;
```

### 2. Asset Preloading

```html
<head>
    <!-- Preload critical assets -->
    <link rel="preload" href="/build/assets/app.js" as="script">
    <link rel="preload" href="/build/assets/app.css" as="style">
</head>
```

### 3. Image Optimization

```rust
// Generate multiple thumbnail sizes
let thumb_small = media.thumbnail(&id, 150, 150).await?;
let thumb_medium = media.thumbnail(&id, 300, 300).await?;
let thumb_large = media.thumbnail(&id, 600, 600).await?;
```

### 4. Lazy Loading

```html
<img src="{{ $image_url }}" loading="lazy" alt="...">
```

---

## Next Steps

1. ✅ Review [PHASE_12_COMPLETE.md](./PHASE_12_COMPLETE.md) for features
2. ✅ Explore [examples/phase12-blog](../examples/phase12-blog/)
3. ✅ Check [examples/phase12-admin](../examples/phase12-admin/)
4. ⏳ Wait for rf-scaffold (code generation)
5. ⏳ Wait for rf-breeze (auth UI)

---

**Questions?** Check the [troubleshooting](#troubleshooting) section or file an issue on GitHub.
