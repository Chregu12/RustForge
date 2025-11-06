use crate::commands::new::config::ProjectConfig;
use color_eyre::Result;
use std::fs;
use std::path::Path;

pub struct ApiRestTemplate {
    config: ProjectConfig,
}

impl ApiRestTemplate {
    pub fn new(config: ProjectConfig) -> Self {
        Self { config }
    }

    /// Generate complete API REST project structure
    pub fn generate(&self, path: &Path) -> Result<()> {
        // Create directory structure
        self.create_directories(path)?;

        // Generate files
        self.generate_cargo_toml(path)?;
        self.generate_env_file(path)?;
        self.generate_gitignore(path)?;
        self.generate_src(path)?;

        if self.config.has_database() {
            self.generate_migrations(path)?;
        }

        if self.config.has_tests() {
            self.generate_tests(path)?;
        }

        Ok(())
    }

    fn create_directories(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join("src"))?;
        fs::create_dir_all(path.join("src/routes"))?;
        fs::create_dir_all(path.join("src/models"))?;
        fs::create_dir_all(path.join("src/middleware"))?;

        if self.config.has_database() {
            fs::create_dir_all(path.join("migrations"))?;
        }

        if self.config.has_tests() {
            fs::create_dir_all(path.join("tests"))?;
        }

        Ok(())
    }

    fn generate_cargo_toml(&self, path: &Path) -> Result<()> {
        let mut dependencies = vec![
            "axum = { version = \"0.7\", features = [\"macros\"] }".to_string(),
            "tokio = { version = \"1\", features = [\"full\"] }".to_string(),
            "serde = { version = \"1\", features = [\"derive\"] }".to_string(),
            "serde_json = \"1\"".to_string(),
            "tower = \"0.4\"".to_string(),
            "tower-http = { version = \"0.5\", features = [\"cors\", \"trace\"] }".to_string(),
            "tracing = \"0.1\"".to_string(),
            "tracing-subscriber = { version = \"0.3\", features = [\"env-filter\"] }".to_string(),
            "color-eyre = \"0.6\"".to_string(),
            "dotenv = \"0.15\"".to_string(),
            "chrono = \"0.4\"".to_string(),
        ];

        if self.config.has_database() {
            dependencies.push("sqlx = { version = \"0.7\", features = [\"postgres\", \"runtime-tokio\", \"chrono\"] }".to_string());
        }

        if self.config.has_auth() {
            dependencies.push("jsonwebtoken = \"9\"".to_string());
            dependencies.push("bcrypt = \"0.15\"".to_string());
        }

        if self.config.has_redis() {
            dependencies.push("redis = { version = \"0.24\", features = [\"tokio-comp\"] }".to_string());
        }

        let mut dev_dependencies = vec![];
        if self.config.has_tests() {
            dev_dependencies.push("rstest = \"0.18\"".to_string());
            dev_dependencies.push("tokio-test = \"0.4\"".to_string());
        }

        let content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}

[dev-dependencies]
{}
"#,
            self.config.name,
            dependencies.join("\n"),
            dev_dependencies.join("\n")
        );

        fs::write(path.join("Cargo.toml"), content)?;
        Ok(())
    }

    fn generate_env_file(&self, path: &Path) -> Result<()> {
        let mut content = String::from("# Server Configuration\n");
        content.push_str("HOST=0.0.0.0\n");
        content.push_str("PORT=3000\n");
        content.push_str("RUST_LOG=info,tower_http=debug\n\n");

        if let Some(db) = &self.config.database {
            content.push_str("# Database Configuration\n");
            content.push_str(&format!("DATABASE_URL={}\n", db.connection_url()));
            content.push_str("\n");
        }

        if self.config.has_auth() {
            content.push_str("# Authentication\n");
            content.push_str("JWT_SECRET=your-secret-key-change-in-production\n");
            content.push_str("JWT_EXPIRATION=86400\n");
            content.push_str("\n");
        }

        if self.config.has_redis() {
            content.push_str("# Redis Configuration\n");
            content.push_str("REDIS_URL=redis://localhost:6379\n");
            content.push_str("\n");
        }

        fs::write(path.join(".env"), content.clone())?;
        fs::write(path.join(".env.example"), content)?;
        Ok(())
    }

    fn generate_gitignore(&self, path: &Path) -> Result<()> {
        let content = r#"# Rust
/target
Cargo.lock

# Environment
.env
.env.local

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db
"#;
        fs::write(path.join(".gitignore"), content)?;
        Ok(())
    }

    fn generate_src(&self, path: &Path) -> Result<()> {
        self.generate_main_rs(path)?;
        self.generate_config_rs(path)?;
        self.generate_error_rs(path)?;
        self.generate_routes_mod(path)?;
        self.generate_health_route(path)?;

        if self.config.has_auth() {
            self.generate_auth_routes(path)?;
            self.generate_auth_middleware(path)?;
        }

        if self.config.has_database() {
            self.generate_database_rs(path)?;
            self.generate_user_model(path)?;
        }

        Ok(())
    }

    fn generate_main_rs(&self, path: &Path) -> Result<()> {
        let db_setup = if self.config.has_database() {
            r#"
    // Database connection
    let db_pool = database::init_pool(&config.database_url).await?;
    tracing::info!("Database connection established");
"#
        } else {
            ""
        };

        let db_layer = if self.config.has_database() {
            ".layer(Extension(db_pool))"
        } else {
            ""
        };

        let auth_routes = if self.config.has_auth() {
            ".merge(routes::auth::routes())"
        } else {
            ""
        };

        let content = format!(
            r#"mod config;
mod error;
mod routes;
{}{}

use axum::{{
    routing::get,
    Router,
    Extension,
}};
use tower_http::{{
    cors::CorsLayer,
    trace::TraceLayer,
}};
use tracing_subscriber::{{layer::SubscriberExt, util::SubscriberInitExt}};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {{
    color_eyre::install()?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "{}=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::from_env()?;
    tracing::info!("Configuration loaded");
{}
    // Build application
    let app = Router::new()
        .route("/", get(|| async {{ "Welcome to {}!" }}))
        .route("/health", get(routes::health::health_check))
        {}
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        {};

    // Start server
    let addr = format!("{{}}:{{}}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server running on http://{{}}", addr);
    println!();
    println!("🚀 Server running on http://{{}}", addr);
    println!("📚 Health check: http://{{}}/health", addr);
    println!();

    axum::serve(listener, app).await?;

    Ok(())
}}
"#,
            if self.config.has_database() { "mod database;\n" } else { "" },
            if self.config.has_database() { "mod models;\n" } else { "" },
            self.config.name,
            db_setup,
            self.config.name,
            auth_routes,
            db_layer
        );

        fs::write(path.join("src/main.rs"), content)?;
        Ok(())
    }

    fn generate_config_rs(&self, path: &Path) -> Result<()> {
        let db_field = if self.config.has_database() {
            "    pub database_url: String,"
        } else {
            ""
        };

        let db_load = if self.config.has_database() {
            r#"
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),"#
        } else {
            ""
        };

        let content = format!(
            r#"use color_eyre::Result;

#[derive(Debug, Clone)]
pub struct Config {{
    pub host: String,
    pub port: u16,
{}
}}

impl Config {{
    pub fn from_env() -> Result<Self> {{
        dotenv::dotenv().ok();

        Ok(Self {{
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a valid number"),{}
        }})
    }}
}}
"#,
            db_field, db_load
        );

        fs::write(path.join("src/config.rs"), content)?;
        Ok(())
    }

    fn generate_error_rs(&self, path: &Path) -> Result<()> {
        let sqlx_impl = if self.config.has_database() {
            r#"

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            _ => AppError::InternalError(err.to_string()),
        }
    }
}"#
        } else {
            ""
        };

        let content = format!(
            r#"use axum::{{
    http::StatusCode,
    response::{{IntoResponse, Response}},
    Json,
}};
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {{
    InternalError(String),
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
}}

impl IntoResponse for AppError {{
    fn into_response(self) -> Response {{
        let (status, message) = match self {{
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        }};

        let body = Json(json!({{
            "error": message,
        }}));

        (status, body).into_response()
    }}
}}{}
"#,
            sqlx_impl
        );

        fs::write(path.join("src/error.rs"), content)?;
        Ok(())
    }

    fn generate_routes_mod(&self, path: &Path) -> Result<()> {
        let auth_mod = if self.config.has_auth() {
            "pub mod auth;"
        } else {
            ""
        };

        let content = format!(
            r#"pub mod health;
{}
"#,
            auth_mod
        );

        fs::write(path.join("src/routes/mod.rs"), content)?;
        Ok(())
    }

    fn generate_health_route(&self, path: &Path) -> Result<()> {
        let content = r#"use axum::Json;
use serde_json::{json, Value};

pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
"#;

        fs::write(path.join("src/routes/health.rs"), content)?;
        Ok(())
    }

    fn generate_auth_routes(&self, path: &Path) -> Result<()> {
        let content = r#"use axum::{
    routing::post,
    Router,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub email: String,
    pub name: String,
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
}

async fn register(
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    // TODO: Implement user registration
    // 1. Hash password with bcrypt
    // 2. Create user in database
    // 3. Generate JWT token

    Ok(Json(AuthResponse {
        token: "dummy_token".to_string(),
        user: UserResponse {
            id: 1,
            email: payload.email,
            name: payload.name,
        },
    }))
}

async fn login(
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // TODO: Implement user login
    // 1. Find user by email
    // 2. Verify password with bcrypt
    // 3. Generate JWT token

    Ok(Json(AuthResponse {
        token: "dummy_token".to_string(),
        user: UserResponse {
            id: 1,
            email: payload.email,
            name: "User".to_string(),
        },
    }))
}
"#;

        fs::write(path.join("src/routes/auth.rs"), content)?;
        Ok(())
    }

    fn generate_auth_middleware(&self, path: &Path) -> Result<()> {
        let content = r#"// JWT authentication middleware
// TODO: Implement JWT token verification
"#;

        fs::write(path.join("src/middleware/auth.rs"), content)?;
        Ok(())
    }

    fn generate_database_rs(&self, path: &Path) -> Result<()> {
        let content = r#"use sqlx::{postgres::PgPoolOptions, PgPool};
use color_eyre::Result;

pub async fn init_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    Ok(pool)
}
"#;

        fs::write(path.join("src/database.rs"), content)?;
        Ok(())
    }

    fn generate_user_model(&self, path: &Path) -> Result<()> {
        let content = r#"use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    // Add user-related methods here
}
"#;

        fs::write(path.join("src/models/mod.rs"), content)?;
        Ok(())
    }

    fn generate_migrations(&self, path: &Path) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        let migration = r#"-- Create users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index on email for faster lookups
CREATE INDEX idx_users_email ON users(email);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at
CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
"#;

        fs::write(
            path.join(format!("migrations/{}_create_users.sql", timestamp)),
            migration,
        )?;
        Ok(())
    }

    fn generate_tests(&self, path: &Path) -> Result<()> {
        let content = r#"#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        // TODO: Implement integration tests
        assert!(true);
    }

    #[tokio::test]
    async fn test_api_endpoints() {
        // TODO: Test API endpoints
        assert!(true);
    }
}
"#;

        fs::write(path.join("tests/integration_test.rs"), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::commands::new::config::{Feature, TemplateType};

    #[test]
    fn test_api_template_creation() {
        let config = ProjectConfig {
            name: "test_app".to_string(),
            path: PathBuf::from("/tmp/test_app"),
            template: TemplateType::ApiRest,
            features: vec![Feature::Database, Feature::Tests],
            database: Some(crate::commands::new::config::DatabaseConfig::default()),
        };

        let template = ApiRestTemplate::new(config);
        assert!(std::mem::size_of_val(&template) > 0);
    }
}
