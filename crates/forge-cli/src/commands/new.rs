use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use fs_extra::dir::{self, CopyOptions};

pub async fn run(name: &str) -> Result<()> {
    println!("{}", "╔══════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                                                      ║".cyan());
    println!("{}", format!("║   🔥 Creating new RustForge project: {:<15}║", name).cyan().bold());
    println!("{}", "║                                                      ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════╝".cyan());
    println!();

    // Validate project name
    if !is_valid_project_name(name) {
        anyhow::bail!("Invalid project name. Use lowercase letters, numbers, hyphens, and underscores only.");
    }

    // Check if directory already exists
    if Path::new(name).exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Find the rustforge-starter template directory
    let starter_template = find_starter_template()?;

    if starter_template.exists() {
        // Copy from rustforge-starter template
        copy_starter_template(&starter_template, name)?;
    } else {
        // Fallback: create basic structure
        println!("{}", "  ⚠ Starter template not found, creating basic structure...".yellow());
        create_project_directory(name)?;
        create_basic_structure(name)?;
    }

    // Customize the project
    customize_project(name)?;

    // Initialize git repository
    init_git_repo(name)?;

    println!();
    println!("{}", "╔══════════════════════════════════════════════════════╗".green());
    println!("{}", "║                                                      ║".green());
    println!("{}", "║   ✓ Project created successfully!                   ║".green().bold());
    println!("{}", "║                                                      ║".green());
    println!("{}", "╚══════════════════════════════════════════════════════╝".green());
    println!();
    println!("{}", "Next steps:".yellow().bold());
    println!("  cd {}", name);
    println!("  cp .env.example .env");
    println!("  forge migrate");
    println!("  cargo run");
    println!();
    println!("{}", "Visit http://localhost:3000 to see your app! 🚀".cyan());
    println!();

    Ok(())
}

fn is_valid_project_name(name: &str) -> bool {
    !name.is_empty() &&
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

fn find_starter_template() -> Result<PathBuf> {
    // Try to find the rustforge-starter directory
    let current_dir = std::env::current_dir()?;

    // Check in current directory
    let local_starter = current_dir.join("rustforge-starter");
    if local_starter.exists() {
        return Ok(local_starter);
    }

    // Check one level up (if we're in the framework repo)
    let parent_starter = current_dir.parent()
        .map(|p| p.join("rustforge-starter"));
    if let Some(path) = parent_starter {
        if path.exists() {
            return Ok(path);
        }
    }

    // Return a non-existent path (will trigger fallback)
    Ok(PathBuf::from("rustforge-starter"))
}

fn copy_starter_template(template_path: &Path, project_name: &str) -> Result<()> {
    println!("  {} Copying starter template...", "•".cyan());

    let options = CopyOptions::new();
    dir::copy(template_path, project_name, &options)
        .context("Failed to copy starter template")?;

    // Remove git directory from template
    let git_dir = Path::new(project_name).join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(git_dir).ok();
    }

    println!("  {} Starter template copied successfully", "✓".green());
    Ok(())
}

fn create_project_directory(name: &str) -> Result<()> {
    fs::create_dir(name)
        .context(format!("Failed to create directory '{}'", name))?;
    Ok(())
}

fn create_basic_structure(name: &str) -> Result<()> {
    let base = Path::new(name);

    // Create directory structure
    println!("  {} Creating directory structure...", "•".cyan());
    fs::create_dir_all(base.join("src"))?;
    fs::create_dir_all(base.join("app/Http/Controllers"))?;
    fs::create_dir_all(base.join("app/Http/Middleware"))?;
    fs::create_dir_all(base.join("app/Models"))?;
    fs::create_dir_all(base.join("app/Services"))?;
    fs::create_dir_all(base.join("config"))?;
    fs::create_dir_all(base.join("database/migrations"))?;
    fs::create_dir_all(base.join("database/seeders"))?;
    fs::create_dir_all(base.join("routes"))?;
    fs::create_dir_all(base.join("resources/views"))?;
    fs::create_dir_all(base.join("public"))?;
    fs::create_dir_all(base.join("storage/logs"))?;
    fs::create_dir_all(base.join("tests"))?;

    // Create Cargo.toml
    println!("  {} Generating Cargo.toml...", "•".cyan());
    let cargo_toml = generate_cargo_toml(name);
    fs::write(base.join("Cargo.toml"), cargo_toml)?;

    // Create .env.example
    println!("  {} Generating .env.example...", "•".cyan());
    let env_content = generate_env_file(name);
    fs::write(base.join(".env.example"), env_content)?;

    // Create .gitignore
    println!("  {} Generating .gitignore...", "•".cyan());
    let gitignore = generate_gitignore();
    fs::write(base.join(".gitignore"), gitignore)?;

    // Create main.rs
    println!("  {} Generating src/main.rs...", "•".cyan());
    let main_rs = generate_main_rs(name);
    fs::write(base.join("src/main.rs"), main_rs)?;

    // Create README.md
    println!("  {} Generating README.md...", "•".cyan());
    let readme = generate_readme(name);
    fs::write(base.join("README.md"), readme)?;

    Ok(())
}

fn customize_project(name: &str) -> Result<()> {
    println!("  {} Customizing project...", "•".cyan());
    let base = Path::new(name);

    // Update Cargo.toml with project name
    let cargo_path = base.join("Cargo.toml");
    if cargo_path.exists() {
        let content = fs::read_to_string(&cargo_path)?;
        let updated = content.replace("my-rustforge-app", name);
        fs::write(&cargo_path, updated)?;
    }

    println!("  {} Project customized", "✓".green());
    Ok(())
}

fn init_git_repo(name: &str) -> Result<()> {
    use std::process::Command;

    println!("  {} Initializing git repository...", "•".cyan());

    let output = Command::new("git")
        .args(&["init", name])
        .output();

    if output.is_ok() {
        println!("  {} Git repository initialized", "✓".green());
    } else {
        println!("  {} Could not initialize git repository", "⚠".yellow());
    }

    Ok(())
}

fn generate_cargo_toml(name: &str) -> String {
    format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Framework (adjust path as needed)
rf-core = {{ path = "../../crates/rf-core" }}
rf-web = {{ path = "../../crates/rf-web" }}
rf-orm = {{ path = "../../crates/rf-orm" }}
rf-auth = {{ path = "../../crates/rf-auth" }}
rf-validation = {{ path = "../../crates/rf-validation" }}
rf-cache = {{ path = "../../crates/rf-cache" }}
rf-queue = {{ path = "../../crates/rf-queue" }}

# Core dependencies
tokio = {{ version = "1.0", features = ["full"] }}
axum = "0.7"
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
sqlx = {{ version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "chrono"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
dotenv = "0.15"
"#, name)
}

fn generate_env_file(name: &str) -> String {
    format!(r#"APP_NAME={}
APP_ENV=local
APP_DEBUG=true
APP_URL=http://localhost:8000

# Database
DATABASE_URL=sqlite:./{}.db

# Redis (optional)
REDIS_URL=redis://127.0.0.1:6379

# Cache
CACHE_DRIVER=memory
CACHE_PREFIX={}_cache_

# Queue
QUEUE_DRIVER=memory
"#, name, name, name)
}

fn generate_gitignore() -> String {
    r#"/target
**/*.rs.bk
Cargo.lock
.env
*.db
*.db-shm
*.db-wal
.DS_Store
"#.to_string()
}

fn generate_main_rs(name: &str) -> String {
    format!("mod db;\n\nuse anyhow::Result;\nuse axum::{{Router, routing::get}};\nuse std::net::SocketAddr;\n\n#[tokio::main]\nasync fn main() -> Result<()> {{\n    // Initialize tracing\n    tracing_subscriber::fmt::init();\n\n    // Load environment variables\n    dotenv::dotenv().ok();\n\n    // Initialize database\n    let pool = db::get_pool().await?;\n    db::run_migrations(&pool).await?;\n\n    // Create router\n    let app = Router::new()\n        .route(\"/\", get(|| async {{ \"Welcome to {}!\" }}));\n\n    // Start server\n    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));\n    println!(\"🚀 Server listening on http://{{}}\", addr);\n\n    let listener = tokio::net::TcpListener::bind(&addr).await?;\n    axum::serve(listener, app).await?;\n\n    Ok(())\n}}\n", name)
}

fn generate_lib_rs() -> String {
    r#"pub mod db;

// Re-export commonly used types
pub use anyhow::Result;
"#.to_string()
}

fn generate_db_rs() -> String {
    r#"use anyhow::Result;
use sqlx::sqlite::SqlitePool;
use std::env;

pub async fn get_pool() -> Result<SqlitePool> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");

    let pool = SqlitePool::connect(&database_url).await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    println!("✓ Running migrations...");

    // Add your migrations here
    // Example:
    // sqlx::query("CREATE TABLE IF NOT EXISTS users (...)").execute(pool).await?;

    println!("✓ Migrations completed");
    Ok(())
}
"#.to_string()
}

fn generate_readme(name: &str) -> String {
    format!(r#"# {}

A RustForge web application.

## Getting Started

1. Install dependencies:
```bash
cargo build
```

2. Run migrations:
```bash
forge migrate
```

3. Start the server:
```bash
forge serve
```

## Development

Generate a model:
```bash
forge make:model User --migration
```

Generate a controller:
```bash
forge make:controller UserController --api
```

## Built with RustForge

RustForge is a Laravel-inspired Rust web framework with:
- Eloquent-like ORM
- Authentication & Authorization
- Queue system
- Real-time broadcasting
- And much more!
"#, name)
}
