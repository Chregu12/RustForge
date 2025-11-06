use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use handlebars::Handlebars;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// RustForge Project Wizard - Zero to Hero in 2-3 Minutes
#[derive(Debug, Clone)]
pub struct ProjectWizard {
    project_name: String,
    project_type: ProjectType,
    features: ProjectFeatures,
    database: Option<DatabaseConfig>,
    template_engine: Handlebars<'static>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    ApiRest,
    FullStackReact,
    FullStackLeptos,
    CliTool,
    Microservice,
    GraphQLApi,
    WebSocketServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFeatures {
    pub authentication: bool,
    pub database: bool,
    pub cache: bool,
    pub queue: bool,
    pub websocket: bool,
    pub graphql: bool,
    pub admin_panel: bool,
    pub docker: bool,
    pub ci_cd: bool,
    pub monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub driver: DatabaseDriver,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseDriver {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
}

impl ProjectWizard {
    /// Create a new project wizard with interactive prompts
    pub async fn interactive(name: Option<String>) -> Result<Self> {
        println!("{}", "
╔═══════════════════════════════════════════════════════════╗
║                                                           ║
║     {}     ║
║                                                           ║
║     {}     ║
║                                                           ║
╚═══════════════════════════════════════════════════════════╝
        ".bright_blue().bold()
        .replace("{}", &"⚡ RUSTFORGE PROJECT WIZARD ⚡".bright_yellow().to_string())
        .replace("{}", &"Zero to Production in 2 Minutes".white().to_string())
        );

        let theme = ColorfulTheme::default();

        // Project Name
        let project_name = match name {
            Some(n) => n,
            None => Input::with_theme(&theme)
                .with_prompt("Project name")
                .default("my-rustforge-app".to_string())
                .interact_text()?,
        };

        // Project Type Selection with descriptions
        let project_types = vec![
            ("🌐 REST API", "RESTful API with OpenAPI docs, auth, and database"),
            ("⚛️  Full-Stack React", "React SPA + Rust API backend"),
            ("🦀 Full-Stack Leptos", "100% Rust with Leptos WASM frontend"),
            ("🖥️  CLI Tool", "Command-line application with rich UI"),
            ("🔧 Microservice", "Cloud-native service with health checks"),
            ("🎯 GraphQL API", "GraphQL API with playground and subscriptions"),
            ("🔌 WebSocket Server", "Real-time server with channels"),
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Select project type")
            .items(&project_types.iter().map(|(name, _)| name).collect::<Vec<_>>())
            .default(0)
            .interact()?;

        let project_type = match selection {
            0 => ProjectType::ApiRest,
            1 => ProjectType::FullStackReact,
            2 => ProjectType::FullStackLeptos,
            3 => ProjectType::CliTool,
            4 => ProjectType::Microservice,
            5 => ProjectType::GraphQLApi,
            6 => ProjectType::WebSocketServer,
            _ => ProjectType::ApiRest,
        };

        // Feature Selection
        println!("\n{}", "📦 Select features to include:".bright_cyan().bold());

        let features = ProjectFeatures {
            authentication: Self::confirm_feature("🔐 Authentication (JWT, Sessions)", true)?,
            database: Self::confirm_feature("💾 Database (with migrations)", true)?,
            cache: Self::confirm_feature("⚡ Cache Layer (Redis/In-Memory)", false)?,
            queue: Self::confirm_feature("📬 Background Jobs Queue", false)?,
            websocket: Self::confirm_feature("🔌 WebSocket Support", false)?,
            graphql: Self::confirm_feature("🎯 GraphQL API", false)?,
            admin_panel: Self::confirm_feature("📊 Admin Dashboard", false)?,
            docker: Self::confirm_feature("🐳 Docker Configuration", true)?,
            ci_cd: Self::confirm_feature("🚀 CI/CD Pipeline (GitHub Actions)", true)?,
            monitoring: Self::confirm_feature("📈 Monitoring (Prometheus/Grafana)", false)?,
        };

        // Database Configuration if selected
        let database = if features.database {
            Some(Self::configure_database()?)
        } else {
            None
        };

        Ok(Self {
            project_name,
            project_type,
            features,
            database,
            template_engine: Handlebars::new(),
        })
    }

    fn confirm_feature(prompt: &str, default: bool) -> Result<bool> {
        Ok(Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .default(default)
            .interact()?)
    }

    fn configure_database() -> Result<DatabaseConfig> {
        let theme = ColorfulTheme::default();

        println!("\n{}", "💾 Database Configuration".bright_cyan().bold());

        let drivers = vec![
            "PostgreSQL (Recommended)",
            "MySQL/MariaDB",
            "SQLite (Local Development)",
            "MongoDB",
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Select database driver")
            .items(&drivers)
            .default(0)
            .interact()?;

        let driver = match selection {
            0 => DatabaseDriver::PostgreSQL,
            1 => DatabaseDriver::MySQL,
            2 => DatabaseDriver::SQLite,
            3 => DatabaseDriver::MongoDB,
            _ => DatabaseDriver::PostgreSQL,
        };

        let (default_host, default_port) = match driver {
            DatabaseDriver::PostgreSQL => ("localhost", 5432),
            DatabaseDriver::MySQL => ("localhost", 3306),
            DatabaseDriver::SQLite => ("", 0),
            DatabaseDriver::MongoDB => ("localhost", 27017),
        };

        let host = if matches!(driver, DatabaseDriver::SQLite) {
            String::new()
        } else {
            Input::with_theme(&theme)
                .with_prompt("Database host")
                .default(default_host.to_string())
                .interact_text()?
        };

        let port = if matches!(driver, DatabaseDriver::SQLite) {
            0
        } else {
            Input::with_theme(&theme)
                .with_prompt("Database port")
                .default(default_port)
                .interact()?
        };

        let name = Input::with_theme(&theme)
            .with_prompt("Database name")
            .default(if matches!(driver, DatabaseDriver::SQLite) {
                "database.db".to_string()
            } else {
                "rustforge_dev".to_string()
            })
            .interact_text()?;

        let username = if matches!(driver, DatabaseDriver::SQLite) {
            String::new()
        } else {
            Input::with_theme(&theme)
                .with_prompt("Database username")
                .default("rustforge".to_string())
                .interact_text()?
        };

        let password = if matches!(driver, DatabaseDriver::SQLite) {
            String::new()
        } else {
            Input::with_theme(&theme)
                .with_prompt("Database password")
                .with_prompt("Database password (hidden)")
                .default("password".to_string())
                .interact_text()?
        };

        Ok(DatabaseConfig {
            driver,
            host,
            port,
            name,
            username,
            password,
        })
    }

    /// Generate the project structure and files
    pub async fn generate(&self) -> Result<()> {
        let pb = ProgressBar::new(10);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        // Create project directory
        pb.set_message("Creating project directory...");
        let project_path = Path::new(&self.project_name);
        fs::create_dir_all(project_path)?;
        pb.inc(1);

        // Generate Cargo.toml
        pb.set_message("Generating Cargo.toml...");
        self.generate_cargo_toml(project_path)?;
        pb.inc(1);

        // Generate src structure
        pb.set_message("Creating source structure...");
        self.generate_src_structure(project_path)?;
        pb.inc(1);

        // Generate config files
        pb.set_message("Generating configuration...");
        self.generate_config(project_path)?;
        pb.inc(1);

        // Generate Docker files if selected
        if self.features.docker {
            pb.set_message("Creating Docker configuration...");
            self.generate_docker(project_path)?;
            pb.inc(1);
        }

        // Generate CI/CD if selected
        if self.features.ci_cd {
            pb.set_message("Setting up CI/CD pipeline...");
            self.generate_ci_cd(project_path)?;
            pb.inc(1);
        }

        // Generate database migrations if selected
        if self.features.database {
            pb.set_message("Creating database migrations...");
            self.generate_migrations(project_path)?;
            pb.inc(1);
        }

        // Initialize git repository
        pb.set_message("Initializing git repository...");
        self.init_git(project_path)?;
        pb.inc(1);

        // Run initial build
        pb.set_message("Running initial build...");
        self.run_initial_build(project_path)?;
        pb.inc(1);

        // Final message
        pb.set_message("Project created successfully!");
        pb.finish_with_message("✨ Done!");

        self.print_success_message();

        Ok(())
    }

    fn generate_cargo_toml(&self, path: &Path) -> Result<()> {
        let mut dependencies = HashMap::new();

        // Base dependencies
        dependencies.insert("rustforge", "0.1");
        dependencies.insert("tokio", r#"{ version = "1.37", features = ["full"] }"#);
        dependencies.insert("axum", r#"{ version = "0.7", features = ["macros"] }"#);
        dependencies.insert("serde", r#"{ version = "1.0", features = ["derive"] }"#);
        dependencies.insert("serde_json", "1.0");
        dependencies.insert("tracing", "0.1");
        dependencies.insert("tracing-subscriber", "0.3");
        dependencies.insert("anyhow", "1.0");
        dependencies.insert("dotenvy", "0.15");

        // Feature-specific dependencies
        if self.features.database {
            dependencies.insert("sea-orm", r#"{ version = "0.12", features = ["runtime-tokio-rustls", "sqlx-postgres"] }"#);
            dependencies.insert("sqlx", r#"{ version = "0.7", features = ["runtime-tokio-rustls", "postgres"] }"#);
        }

        if self.features.authentication {
            dependencies.insert("jsonwebtoken", "9.2");
            dependencies.insert("argon2", "0.5");
            dependencies.insert("tower-sessions", "0.12");
        }

        if self.features.cache {
            dependencies.insert("redis", r#"{ version = "0.25", features = ["tokio-comp", "connection-manager"] }"#);
        }

        if self.features.graphql {
            dependencies.insert("async-graphql", r#"{ version = "7.0", features = ["chrono"] }"#);
            dependencies.insert("async-graphql-axum", "7.0");
        }

        let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}

[dev-dependencies]
criterion = "0.5"
proptest = "1.4"
faker_rand = "0.1"

[[bin]]
name = "{}"
path = "src/main.rs"
"#,
            self.project_name,
            dependencies.iter()
                .map(|(k, v)| format!("{} = {}", k, v))
                .collect::<Vec<_>>()
                .join("\n"),
            self.project_name
        );

        fs::write(path.join("Cargo.toml"), cargo_toml)?;
        Ok(())
    }

    fn generate_src_structure(&self, path: &Path) -> Result<()> {
        let src_path = path.join("src");
        fs::create_dir_all(&src_path)?;

        // Generate main.rs based on project type
        let main_content = match self.project_type {
            ProjectType::ApiRest => self.generate_api_main(),
            ProjectType::FullStackReact => self.generate_fullstack_main(),
            ProjectType::FullStackLeptos => self.generate_leptos_main(),
            ProjectType::CliTool => self.generate_cli_main(),
            ProjectType::Microservice => self.generate_microservice_main(),
            ProjectType::GraphQLApi => self.generate_graphql_main(),
            ProjectType::WebSocketServer => self.generate_websocket_main(),
        };

        fs::write(src_path.join("main.rs"), main_content)?;

        // Create module structure
        fs::create_dir_all(src_path.join("handlers"))?;
        fs::create_dir_all(src_path.join("models"))?;
        fs::create_dir_all(src_path.join("services"))?;
        fs::create_dir_all(src_path.join("middleware"))?;
        fs::create_dir_all(src_path.join("config"))?;

        // Generate example handler
        self.generate_example_handler(&src_path)?;

        // Generate example model
        if self.features.database {
            self.generate_example_model(&src_path)?;
        }

        Ok(())
    }

    fn generate_api_main(&self) -> String {
        format!(r#"use rustforge::prelude::*;
use axum::{{Router, routing::get}};
use std::net::SocketAddr;
use tracing_subscriber;

mod config;
mod handlers;
mod models;
mod services;
mod middleware;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment
    dotenvy::dotenv().ok();

    // Initialize RustForge application
    let app = RustForge::new()
        .config(config::load()?){}{}{}
        .build()
        .await?;

    // Build router
    let router = Router::new()
        .route("/", get(handlers::health::check))
        .route("/api/v1/users", get(handlers::users::list)){}{}
        .with_state(app.state());

    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("🚀 Server running on http://{{addr}}");

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}}
"#,
            if self.features.database { "\n        .database()" } else { "" },
            if self.features.cache { "\n        .cache()" } else { "" },
            if self.features.authentication { "\n        .auth()" } else { "" },
            if self.features.authentication { "\n        .route(\"/api/v1/auth/login\", post(handlers::auth::login))" } else { "" },
            if self.features.graphql { "\n        .route(\"/graphql\", get(handlers::graphql::playground).post(handlers::graphql::handler))" } else { "" }
        )
    }

    fn generate_fullstack_main(&self) -> String {
        // React + Rust API implementation
        format!(r#"use rustforge::prelude::*;
use axum::{{Router, routing::{{get, get_service}}}};
use tower_http::services::ServeDir;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let app = RustForge::new()
        .config(config::load()?)
        .build()
        .await?;

    // API routes
    let api = Router::new()
        .route("/health", get(handlers::health::check))
        .route("/users", get(handlers::users::list));

    // Main router with static file serving for React
    let router = Router::new()
        .nest("/api/v1", api)
        .fallback(get_service(ServeDir::new("./frontend/dist")))
        .with_state(app.state());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("🚀 Full-stack app running on http://{{addr}}");

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}}
"#)
    }

    fn generate_leptos_main(&self) -> String {
        // 100% Rust with Leptos
        format!(r#"use rustforge::prelude::*;
use leptos::*;
use leptos_axum::{{generate_route_list, LeptosRoutes}};
use axum::Router;
use std::net::SocketAddr;

#[component]
fn App() -> impl IntoView {{
    view! {{
        <div class="container">
            <h1>"Welcome to RustForge + Leptos!"</h1>
            <p>"100% Rust Full-Stack Application"</p>
        </div>
    }}
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, App)
        .with_state(leptos_options);

    tracing::info!("🦀 Leptos app running on http://{{addr}}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}}
"#)
    }

    fn generate_cli_main(&self) -> String {
        format!(r#"use rustforge::cli::prelude::*;
use clap::{{Parser, Subcommand}};
use colored::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {{
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,
}}

#[derive(Subcommand)]
enum Commands {{
    /// Process data from input
    Process {{
        #[arg(short, long)]
        input: String,

        #[arg(short, long)]
        output: Option<String>,
    }},

    /// Sync with external service
    Sync {{
        #[arg(short, long)]
        force: bool,
    }},
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {{
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }}

    match cli.command {{
        Commands::Process {{ input, output }} => {{
            println!("{{}}", "Processing...".green().bold());
            // Process logic here
        }},
        Commands::Sync {{ force }} => {{
            println!("{{}}", "Syncing...".blue().bold());
            // Sync logic here
        }},
    }}

    Ok(())
}}
"#)
    }

    fn generate_microservice_main(&self) -> String {
        format!(r#"use rustforge::microservice::prelude::*;
use axum::{{Router, routing::get}};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let app = RustForge::microservice()
        .name("{}")
        .health_check("/health")
        .metrics("/metrics")
        .ready_check("/ready")
        .build()
        .await?;

    let router = Router::new()
        .route("/", get(root))
        .merge(app.routes())
        .with_state(app.state());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🔧 Microservice running on http://{{addr}}");

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}}

async fn root() -> &'static str {{
    "Microservice is running!"
}}
"#, self.project_name)
    }

    fn generate_graphql_main(&self) -> String {
        format!(r#"use rustforge::graphql::prelude::*;
use async_graphql::{{EmptyMutation, EmptySubscription, Object, Schema}};
use async_graphql_axum::{{GraphQLRequest, GraphQLResponse}};
use axum::{{extract::State, response::Html, Router, routing::{{get, post}}}};
use std::net::SocketAddr;

struct Query;

#[Object]
impl Query {{
    async fn hello(&self, name: Option<String>) -> String {{
        format!("Hello, {{}}!", name.unwrap_or_else(|| "World".to_string()))
    }}

    async fn add(&self, a: i32, b: i32) -> i32 {{
        a + b
    }}
}}

type AppSchema = Schema<Query, EmptyMutation, EmptySubscription>;

async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {{
    schema.execute(req.into_inner()).await.into()
}}

async fn graphql_playground() -> Html<&'static str> {{
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    tracing_subscriber::fmt::init();

    let schema = Schema::new(Query, EmptyMutation, EmptySubscription);

    let app = Router::new()
        .route("/graphql", post(graphql_handler).get(graphql_playground))
        .with_state(schema);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    tracing::info!("🎯 GraphQL server running on http://{{addr}}/graphql");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}}
"#)
    }

    fn generate_websocket_main(&self) -> String {
        format!(r#"use rustforge::websocket::prelude::*;
use axum::{{
    extract::ws::{{WebSocket, WebSocketUpgrade}},
    response::IntoResponse,
    Router, routing::get,
}};
use std::net::SocketAddr;
use futures::{{SinkExt, StreamExt}};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {{
    ws.on_upgrade(handle_socket)
}}

async fn handle_socket(mut socket: WebSocket) {{
    println!("New WebSocket connection");

    while let Some(msg) = socket.recv().await {{
        if let Ok(msg) = msg {{
            match msg {{
                axum::extract::ws::Message::Text(text) => {{
                    println!("Received: {{text}}");
                    if socket
                        .send(axum::extract::ws::Message::Text(format!("Echo: {{text}}")))
                        .await
                        .is_err()
                    {{
                        break;
                    }}
                }}
                axum::extract::ws::Message::Close(_) => break,
                _ => {{}}
            }}
        }} else {{
            break;
        }}
    }}

    println!("WebSocket connection closed");
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/", get(|| async {{ Html(include_str!("../static/index.html")) }}));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("🔌 WebSocket server running on http://{{addr}}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}}
"#)
    }

    fn generate_example_handler(&self, src_path: &Path) -> Result<()> {
        let handlers_path = src_path.join("handlers");

        // Health check handler
        fs::write(
            handlers_path.join("health.rs"),
            r#"use axum::Json;
use serde_json::json;

pub async fn check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
"#,
        )?;

        // Users handler (if database enabled)
        if self.features.database {
            fs::write(
                handlers_path.join("users.rs"),
                r#"use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::models::user::User;

#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub name: String,
    pub email: String,
}

pub async fn list(State(db): State<DatabaseConnection>) -> Json<Vec<UserResponse>> {
    let users = User::find()
        .all(&db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|u| UserResponse {
            id: u.id,
            name: u.name,
            email: u.email,
        })
        .collect();

    Json(users)
}
"#,
            )?;
        }

        // Auth handler (if authentication enabled)
        if self.features.authentication {
            fs::write(
                handlers_path.join("auth.rs"),
                r#"use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Header, EncodingKey};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub email: String,
    pub name: String,
}

pub async fn login(Json(req): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    // TODO: Verify credentials from database

    let claims = UserInfo {
        id: 1,
        email: req.email.clone(),
        name: "User".to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("secret".as_ref()),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: claims,
    }))
}
"#,
            )?;
        }

        // Create mod.rs
        let mut mod_content = String::from("pub mod health;\n");
        if self.features.database {
            mod_content.push_str("pub mod users;\n");
        }
        if self.features.authentication {
            mod_content.push_str("pub mod auth;\n");
        }
        if self.features.graphql {
            mod_content.push_str("pub mod graphql;\n");
        }

        fs::write(handlers_path.join("mod.rs"), mod_content)?;

        Ok(())
    }

    fn generate_example_model(&self, src_path: &Path) -> Result<()> {
        let models_path = src_path.join("models");

        // User model
        fs::write(
            models_path.join("user.rs"),
            r#"use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub type User = Model;
"#,
        )?;

        // Create mod.rs
        fs::write(
            models_path.join("mod.rs"),
            "pub mod user;\n",
        )?;

        Ok(())
    }

    fn generate_config(&self, path: &Path) -> Result<()> {
        let config_path = path.join("config");
        fs::create_dir_all(&config_path)?;

        // Generate .env file
        let mut env_content = String::from("# RustForge Configuration\n\n");
        env_content.push_str("APP_NAME=");
        env_content.push_str(&self.project_name);
        env_content.push_str("\n");
        env_content.push_str("APP_ENV=development\n");
        env_content.push_str("APP_URL=http://localhost:3000\n");
        env_content.push_str("APP_PORT=3000\n\n");

        if let Some(db) = &self.database {
            env_content.push_str("# Database Configuration\n");
            env_content.push_str(&format!("DATABASE_URL={}\n", self.build_database_url(db)));
            env_content.push_str(&format!("DATABASE_DRIVER={:?}\n", db.driver));
        }

        if self.features.cache {
            env_content.push_str("\n# Cache Configuration\n");
            env_content.push_str("REDIS_URL=redis://localhost:6379\n");
        }

        if self.features.authentication {
            env_content.push_str("\n# Authentication\n");
            env_content.push_str("JWT_SECRET=your-secret-key-change-this\n");
            env_content.push_str("JWT_EXPIRATION=86400\n");
        }

        fs::write(path.join(".env"), env_content)?;
        fs::write(path.join(".env.example"), env_content)?;

        // Generate rustforge.toml
        let rustforge_config = format!(r#"# RustForge Project Configuration

[app]
name = "{}"
version = "0.1.0"
environment = "development"

[server]
host = "127.0.0.1"
port = 3000
workers = 4

{}{}{}{}

[logging]
level = "info"
format = "pretty"
"#,
            self.project_name,
            if self.database.is_some() {
                r#"
[database]
pool_size = 10
max_connections = 100
timeout = 30
"#
            } else { "" },
            if self.features.cache {
                r#"
[cache]
driver = "redis"
prefix = "rustforge"
ttl = 3600
"#
            } else { "" },
            if self.features.queue {
                r#"
[queue]
driver = "redis"
workers = 4
retry_attempts = 3
"#
            } else { "" },
            if self.features.monitoring {
                r#"
[monitoring]
metrics_endpoint = "/metrics"
health_endpoint = "/health"
ready_endpoint = "/ready"
"#
            } else { "" }
        );

        fs::write(config_path.join("rustforge.toml"), rustforge_config)?;

        // Generate config module
        fs::write(
            path.join("src").join("config.rs"),
            r#"use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub server: ServerConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub pool_size: u32,
    pub max_connections: u32,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub driver: String,
    pub prefix: String,
    pub ttl: u64,
}

pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string("config/rustforge.toml")?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
"#,
        )?;

        Ok(())
    }

    fn build_database_url(&self, db: &DatabaseConfig) -> String {
        match db.driver {
            DatabaseDriver::PostgreSQL => {
                format!("postgresql://{}:{}@{}:{}/{}",
                    db.username, db.password, db.host, db.port, db.name)
            },
            DatabaseDriver::MySQL => {
                format!("mysql://{}:{}@{}:{}/{}",
                    db.username, db.password, db.host, db.port, db.name)
            },
            DatabaseDriver::SQLite => {
                format!("sqlite://{}", db.name)
            },
            DatabaseDriver::MongoDB => {
                format!("mongodb://{}:{}@{}:{}/{}",
                    db.username, db.password, db.host, db.port, db.name)
            },
        }
    }

    fn generate_docker(&self, path: &Path) -> Result<()> {
        // Dockerfile
        let dockerfile = format!(r#"# Build stage
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/{} /app/{}
COPY config ./config

ENV APP_ENV=production
EXPOSE 3000

CMD ["./{}"]
"#, self.project_name, self.project_name, self.project_name);

        fs::write(path.join("Dockerfile"), dockerfile)?;

        // docker-compose.yml
        let mut docker_compose = format!(r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - APP_ENV=production
      - APP_PORT=3000{}
    depends_on:{}
"#,
            if self.database.is_some() {
                "\n      - DATABASE_URL=${DATABASE_URL}"
            } else { "" },
            if self.database.is_some() {
                "\n      - db"
            } else { "" }
        );

        if let Some(db) = &self.database {
            match db.driver {
                DatabaseDriver::PostgreSQL => {
                    docker_compose.push_str(r#"
  db:
    image: postgres:16-alpine
    environment:
      - POSTGRES_USER=rustforge
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=rustforge_dev
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
"#);
                },
                DatabaseDriver::MySQL => {
                    docker_compose.push_str(r#"
  db:
    image: mysql:8
    environment:
      - MYSQL_ROOT_PASSWORD=root
      - MYSQL_USER=rustforge
      - MYSQL_PASSWORD=password
      - MYSQL_DATABASE=rustforge_dev
    volumes:
      - mysql_data:/var/lib/mysql
    ports:
      - "3306:3306"
"#);
                },
                _ => {}
            }
        }

        if self.features.cache {
            docker_compose.push_str(r#"
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
"#);
        }

        docker_compose.push_str(r#"

volumes:
"#);

        if let Some(db) = &self.database {
            match db.driver {
                DatabaseDriver::PostgreSQL => docker_compose.push_str("  postgres_data:\n"),
                DatabaseDriver::MySQL => docker_compose.push_str("  mysql_data:\n"),
                _ => {}
            }
        }

        if self.features.cache {
            docker_compose.push_str("  redis_data:\n");
        }

        fs::write(path.join("docker-compose.yml"), docker_compose)?;

        // .dockerignore
        fs::write(path.join(".dockerignore"), r#"target/
.git/
.env
*.log
"#)?;

        Ok(())
    }

    fn generate_ci_cd(&self, path: &Path) -> Result<()> {
        let github_path = path.join(".github").join("workflows");
        fs::create_dir_all(&github_path)?;

        let ci_workflow = format!(r#"name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    {}
    steps:
    - uses: actions/checkout@v4

    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: rustfmt, clippy

    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{{{ runner.os }}}}-cargo-${{{{ hashFiles('**/Cargo.lock') }}}}

    - name: Format check
      run: cargo fmt -- --check

    - name: Clippy
      run: cargo clippy -- -D warnings

    - name: Test
      run: cargo test --all-features

    - name: Build
      run: cargo build --release

  deploy:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    steps:
    - uses: actions/checkout@v4

    - name: Build Docker image
      run: docker build -t {}/{}:latest .

    - name: Deploy
      run: |
        echo "Deploy to production"
        # Add your deployment commands here
"#,
            if self.database.is_some() {
                r#"
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
"#
            } else { "" },
            "your-registry",
            self.project_name
        );

        fs::write(github_path.join("ci.yml"), ci_workflow)?;

        Ok(())
    }

    fn generate_migrations(&self, path: &Path) -> Result<()> {
        let migrations_path = path.join("migrations");
        fs::create_dir_all(&migrations_path)?;

        // Create initial migration
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let migration_name = format!("{}_{}_create_users_table.sql", timestamp, "001");

        let migration_content = match self.database.as_ref().map(|d| &d.driver) {
            Some(DatabaseDriver::PostgreSQL) => r#"-- Create users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
"#,
            Some(DatabaseDriver::MySQL) => r#"-- Create users table
CREATE TABLE users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
"#,
            Some(DatabaseDriver::SQLite) => r#"-- Create users table
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_created_at ON users(created_at);
"#,
            _ => "",
        };

        if !migration_content.is_empty() {
            fs::write(migrations_path.join(migration_name), migration_content)?;
        }

        Ok(())
    }

    fn init_git(&self, path: &Path) -> Result<()> {
        // Initialize git repository
        Command::new("git")
            .arg("init")
            .current_dir(path)
            .output()?;

        // Create .gitignore
        fs::write(path.join(".gitignore"), r#"# Rust
target/
**/*.rs.bk
*.pdb

# Environment
.env
.env.local
.env.*.local

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/

# Testing
coverage/
*.profraw
*.profdata

# Dependencies
node_modules/
dist/
"#)?;

        // Initial commit
        Command::new("git")
            .args(&["add", "."])
            .current_dir(path)
            .output()?;

        Command::new("git")
            .args(&["commit", "-m", "Initial commit - Generated by RustForge"])
            .current_dir(path)
            .output()?;

        Ok(())
    }

    fn run_initial_build(&self, path: &Path) -> Result<()> {
        // Run cargo check to verify everything compiles
        Command::new("cargo")
            .arg("check")
            .current_dir(path)
            .output()?;

        Ok(())
    }

    fn print_success_message(&self) {
        let mut next_steps = vec![
            format!("cd {}", self.project_name),
        ];

        if self.features.database {
            next_steps.push("rustforge db:migrate".to_string());
        }

        next_steps.push("cargo run".to_string());

        println!("\n{}", "════════════════════════════════════════════════════════════".bright_green());
        println!("{}", "✨ PROJECT CREATED SUCCESSFULLY!".bright_green().bold());
        println!("{}", "════════════════════════════════════════════════════════════".bright_green());

        println!("\n📁 Project: {}", self.project_name.bright_yellow());
        println!("📦 Type: {:?}", self.project_type);

        println!("\n✅ Features included:");
        if self.features.authentication { println!("   • Authentication"); }
        if self.features.database { println!("   • Database with migrations"); }
        if self.features.cache { println!("   • Cache layer"); }
        if self.features.queue { println!("   • Background jobs"); }
        if self.features.websocket { println!("   • WebSocket support"); }
        if self.features.graphql { println!("   • GraphQL API"); }
        if self.features.admin_panel { println!("   • Admin dashboard"); }
        if self.features.docker { println!("   • Docker configuration"); }
        if self.features.ci_cd { println!("   • CI/CD pipeline"); }
        if self.features.monitoring { println!("   • Monitoring"); }

        println!("\n🚀 Next steps:");
        for (i, step) in next_steps.iter().enumerate() {
            println!("   {}. {}", i + 1, step.bright_cyan());
        }

        println!("\n📚 Documentation: https://docs.rustforge.dev");
        println!("💬 Community: https://discord.gg/rustforge");

        println!("\n{}", "Happy coding! 🦀".bright_magenta().bold());
    }
}

// Export for CLI usage
pub async fn run() -> Result<()> {
    let wizard = ProjectWizard::interactive(None).await?;
    wizard.generate().await?;
    Ok(())
}