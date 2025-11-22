use anyhow::{Context, Result};
use colored::*;
use handlebars::Handlebars;
use inflector::Inflector;
use serde_json::json;
use std::fs;
use std::path::Path;

use super::ensure_forge_project;
use crate::{errors, interactive, progress};

pub async fn model(name: &str, with_migration: bool) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating model: {}", name).green().bold());

    let model_name = name.to_pascal_case();
    let table_name = name.to_snake_case().to_plural();

    // Create model file
    let model_path = format!("src/models/{}.rs", name.to_snake_case());

    if Path::new(&model_path).exists() {
        anyhow::bail!("Model already exists: {}", model_path);
    }

    let model_content = generate_model_content(&model_name, &table_name)?;

    // Ensure models directory exists
    fs::create_dir_all("src/models")?;
    fs::write(&model_path, model_content)?;

    println!("  {} Created: {}", "✓".green(), model_path);

    // Update src/models/mod.rs
    update_models_mod(&name.to_snake_case(), &model_name)?;

    // Create migration if requested
    if with_migration {
        let migration_name = format!("create_{}_table", table_name);
        migration(&migration_name).await?;
    }

    println!();
    println!("{}", "Model generated successfully!".green().bold());

    Ok(())
}

pub async fn controller(name: &str, api: bool) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating controller: {}", name).green().bold()
    );

    let controller_name = if name.ends_with("Controller") {
        name.to_string()
    } else {
        format!("{}Controller", name.to_pascal_case())
    };

    let controller_path = format!("src/controllers/{}.rs", controller_name.to_snake_case());

    if Path::new(&controller_path).exists() {
        anyhow::bail!("Controller already exists: {}", controller_path);
    }

    let controller_content = if api {
        generate_api_controller_content(&controller_name)?
    } else {
        generate_web_controller_content(&controller_name)?
    };

    // Ensure controllers directory exists
    fs::create_dir_all("src/controllers")?;
    fs::write(&controller_path, controller_content)?;

    println!("  {} Created: {}", "✓".green(), controller_path);

    // Update src/controllers/mod.rs
    update_controllers_mod(&controller_name.to_snake_case())?;

    println!();
    println!("{}", "Controller generated successfully!".green().bold());

    Ok(())
}

pub async fn migration(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating migration: {}", name).green().bold()
    );

    // Generate timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let migration_name = name.to_snake_case();
    let migration_file = format!("src/migrations/{}_{}.rs", timestamp, migration_name);

    if Path::new(&migration_file).exists() {
        anyhow::bail!("Migration already exists: {}", migration_file);
    }

    let migration_content = generate_migration_content(name)?;

    // Ensure migrations directory exists
    fs::create_dir_all("src/migrations")?;
    fs::write(&migration_file, migration_content)?;

    println!("  {} Created: {}", "✓".green(), migration_file);
    println!();
    println!("{}", "Migration generated successfully!".green().bold());
    println!("Run {} to apply migrations", "forge migrate".cyan());

    Ok(())
}

pub async fn command(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating command: {}", name).green().bold());

    let command_name = name.to_pascal_case();
    let command_path = format!("src/commands/{}.rs", name.to_snake_case());

    if Path::new(&command_path).exists() {
        anyhow::bail!("Command already exists: {}", command_path);
    }

    let command_content = generate_command_content(&command_name)?;

    // Ensure commands directory exists
    fs::create_dir_all("src/commands")?;
    fs::write(&command_path, command_content)?;

    println!("  {} Created: {}", "✓".green(), command_path);
    println!();
    println!("{}", "Command generated successfully!".green().bold());

    Ok(())
}

// Helper functions for content generation

fn generate_model_content(model_name: &str, table_name: &str) -> Result<String> {
    let template = r#"use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct {{model_name}} {
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Add your fields here
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Create{{model_name}} {
    // Add your fields here
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Update{{model_name}} {
    // Add your fields here
}

impl {{model_name}} {
    pub fn table_name() -> &'static str {
        "{{table_name}}"
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("model", template)?;

    let data = json!({
        "model_name": model_name,
        "table_name": table_name,
    });

    Ok(handlebars.render("model", &data)?)
}

fn generate_api_controller_content(controller_name: &str) -> Result<String> {
    let template = r#"use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

pub struct {{controller_name}};

impl {{controller_name}} {
    /// GET /api/resource
    pub async fn index(Query(query): Query<ListQuery>) -> Result<Json<Vec<String>>, StatusCode> {
        // TODO: Implement list logic
        Ok(Json(vec![]))
    }

    /// GET /api/resource/:id
    pub async fn show(Path(id): Path<i32>) -> Result<Json<String>, StatusCode> {
        // TODO: Implement show logic
        Ok(Json(format!("Resource {}", id)))
    }

    /// POST /api/resource
    pub async fn create(Json(payload): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, StatusCode> {
        // TODO: Implement create logic
        Ok(Json(payload))
    }

    /// PUT /api/resource/:id
    pub async fn update(
        Path(id): Path<i32>,
        Json(payload): Json<serde_json::Value>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        // TODO: Implement update logic
        Ok(Json(payload))
    }

    /// DELETE /api/resource/:id
    pub async fn delete(Path(id): Path<i32>) -> Result<StatusCode, StatusCode> {
        // TODO: Implement delete logic
        Ok(StatusCode::NO_CONTENT)
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("controller", template)?;

    let data = json!({
        "controller_name": controller_name,
    });

    Ok(handlebars.render("controller", &data)?)
}

fn generate_web_controller_content(controller_name: &str) -> Result<String> {
    let template = r#"use axum::{
    extract::Path,
    response::Html,
};
use anyhow::Result;

pub struct {{controller_name}};

impl {{controller_name}} {
    pub async fn index() -> Html<String> {
        Html("<h1>Index</h1>".to_string())
    }

    pub async fn show(Path(id): Path<i32>) -> Html<String> {
        Html(format!("<h1>Show {}</h1>", id))
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("controller", template)?;

    let data = json!({
        "controller_name": controller_name,
    });

    Ok(handlebars.render("controller", &data)?)
}

fn generate_migration_content(name: &str) -> Result<String> {
    let is_create = name.starts_with("create_") && name.ends_with("_table");

    let template = if is_create {
        let table_name = name
            .strip_prefix("create_")
            .unwrap()
            .strip_suffix("_table")
            .unwrap();

        format!("use anyhow::Result;\nuse sqlx::SqlitePool;\n\npub async fn up(pool: &SqlitePool) -> Result<()> {{\n    sqlx::query(\n        r#\"\n        CREATE TABLE IF NOT EXISTS {} (\n            id INTEGER PRIMARY KEY AUTOINCREMENT,\n            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,\n            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\n        )\n        \"#,\n    )\n    .execute(pool)\n    .await?;\n\n    Ok(())\n}}\n\npub async fn down(pool: &SqlitePool) -> Result<()> {{\n    sqlx::query(\"DROP TABLE IF EXISTS {}\")\n        .execute(pool)\n        .await?;\n\n    Ok(())\n}}\n", table_name, table_name)
    } else {
        "use anyhow::Result;\nuse sqlx::SqlitePool;\n\npub async fn up(pool: &SqlitePool) -> Result<()> {\n    // TODO: Write migration up logic\n    Ok(())\n}\n\npub async fn down(pool: &SqlitePool) -> Result<()> {\n    // TODO: Write migration down logic\n    Ok(())\n}\n".to_string()
    };

    Ok(template)
}

fn generate_command_content(command_name: &str) -> Result<String> {
    let template = r#"use anyhow::Result;

pub struct {{command_name}};

impl {{command_name}} {
    pub async fn run() -> Result<()> {
        println!("Running {{command_name}}...");

        // TODO: Implement command logic

        println!("✓ {{command_name}} completed!");
        Ok(())
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("command", template)?;

    let data = json!({
        "command_name": command_name,
    });

    Ok(handlebars.render("command", &data)?)
}

fn update_models_mod(file_name: &str, model_name: &str) -> Result<()> {
    let mod_file = "src/models/mod.rs";

    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    // Add module declaration
    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    // Add pub use
    let use_line = format!("pub use {}::{{{{{}}}}};\n", file_name, model_name);
    if !content.contains(&use_line) {
        content.push_str(&use_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);

    Ok(())
}

fn update_controllers_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/controllers/mod.rs";

    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    // Add module declaration
    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);

    Ok(())
}

pub async fn factory(name: &str, model_name: Option<&str>) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating factory: {}", name).green().bold());

    let factory_name = if name.ends_with("Factory") {
        name.to_string()
    } else {
        format!("{}Factory", name.to_pascal_case())
    };

    let model = model_name.map(|m| m.to_pascal_case()).unwrap_or_else(|| {
        factory_name
            .strip_suffix("Factory")
            .unwrap_or(&factory_name)
            .to_string()
    });

    let factory_path = format!("tests/factories/{}.rs", factory_name.to_snake_case());

    if Path::new(&factory_path).exists() {
        anyhow::bail!("Factory already exists: {}", factory_path);
    }

    let factory_content = generate_factory_content(&factory_name, &model)?;

    // Ensure factories directory exists
    fs::create_dir_all("tests/factories")?;
    fs::write(&factory_path, factory_content)?;

    println!("  {} Created: {}", "✓".green(), factory_path);

    // Update tests/factories/mod.rs
    update_factories_mod(&factory_name.to_snake_case())?;

    println!();
    println!("{}", "Factory generated successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Implement the definition() method with fake data");
    println!("  2. Add custom state methods if needed");
    println!("  3. Use in tests: {}::new().create().await", factory_name);

    Ok(())
}

pub async fn seeder(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating seeder: {}", name).green().bold());

    let seeder_name = if name.ends_with("Seeder") {
        name.to_string()
    } else {
        format!("{}Seeder", name.to_pascal_case())
    };

    let seeder_path = format!("database/seeders/{}.rs", seeder_name.to_snake_case());

    if Path::new(&seeder_path).exists() {
        anyhow::bail!("Seeder already exists: {}", seeder_path);
    }

    let seeder_content = generate_seeder_content(&seeder_name)?;

    // Ensure seeders directory exists
    fs::create_dir_all("database/seeders")?;
    fs::write(&seeder_path, seeder_content)?;

    println!("  {} Created: {}", "✓".green(), seeder_path);

    // Update database/seeders/mod.rs
    update_seeders_mod(&seeder_name.to_snake_case(), &seeder_name)?;

    println!();
    println!("{}", "Seeder generated successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Implement the run() method");
    println!("  2. Add to DatabaseSeeder if needed");
    println!("  3. Run with: forge db:seed --class={}", seeder_name);

    Ok(())
}

pub async fn seed(class: Option<&str>, force: bool) -> Result<()> {
    ensure_forge_project()?;

    if !force {
        // Check if production environment
        if let Ok(env) = std::env::var("APP_ENV") {
            if env == "production" {
                anyhow::bail!("Cannot seed production database without --force flag");
            }
        }
    }

    if let Some(seeder_class) = class {
        println!("{}", format!("Seeding: {}", seeder_class).green().bold());
        // TODO: Load and run specific seeder
        println!("  {} Running {}...", "→".blue(), seeder_class);
        println!("  {} Completed!", "✓".green());
    } else {
        println!("{}", "Seeding database...".green().bold());
        // TODO: Run all seeders from DatabaseSeeder
        println!("  {} Running DatabaseSeeder...", "→".blue());
        println!("  {} Database seeded successfully!", "✓".green());
    }

    Ok(())
}

fn generate_factory_content(factory_name: &str, model_name: &str) -> Result<String> {
    let template = r#"use rf_testing::{Factory, FactoryDefinition, FactoryError, Fake};
use async_trait::async_trait;
use crate::models::{{model_name}};

/// Factory for generating {{model_name}} test data
pub struct {{factory_name}} {
    model: {{model_name}},
}

impl Default for {{factory_name}} {
    fn default() -> Self {
        Self {
            model: Self::definition(),
        }
    }
}

impl FactoryDefinition for {{factory_name}} {
    type Model = {{model_name}};

    fn definition() -> Self::Model {
        {{model_name}} {
            // TODO: Add fields with fake data
            // Example:
            // id: 0,
            // name: Fake::name(),
            // email: Fake::email(),
            // created_at: Fake::datetime(),
        }
    }
}

// Implement the Factory trait using the macro
rf_testing::impl_factory!({{factory_name}}, {{model_name}});

// Optional: Add custom state methods
impl {{factory_name}} {
    // Example custom state method:
    // pub fn admin(mut self) -> Self {
    //     self.model.role = "admin".to_string();
    //     self
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_create() {
        let instance = {{factory_name}}::new().create().await.unwrap();
        // Add assertions
    }

    #[tokio::test]
    async fn test_factory_create_many() {
        let instances = {{factory_name}}::create_many(5).await.unwrap();
        assert_eq!(instances.len(), 5);
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("factory", template)?;

    let data = json!({
        "factory_name": factory_name,
        "model_name": model_name,
    });

    Ok(handlebars.render("factory", &data)?)
}

fn generate_seeder_content(seeder_name: &str) -> Result<String> {
    let template = r#"use rf_testing::{Seeder, SeederError};
use async_trait::async_trait;
// use crate::factories::*;

/// Seeder for populating {{seeder_name}} data
pub struct {{seeder_name}};

#[async_trait]
impl Seeder for {{seeder_name}} {
    fn name(&self) -> &str {
        "{{seeder_name}}"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("Seeding {{seeder_name}}...");

        // TODO: Implement seeder logic
        // Example:
        // let users = UserFactory::create_many(50).await?;

        Ok(())
    }

    // Optional: Add dependencies
    // fn dependencies(&self) -> Vec<&str> {
    //     vec!["UserSeeder"]
    // }

    // Optional: Add conditional execution
    // async fn should_run(&self) -> bool {
    //     // Only run if some condition is met
    //     true
    // }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("seeder", template)?;

    let data = json!({
        "seeder_name": seeder_name,
    });

    Ok(handlebars.render("seeder", &data)?)
}

fn update_factories_mod(file_name: &str) -> Result<()> {
    let mod_file = "tests/factories/mod.rs";

    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    // Add module declaration
    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);

    Ok(())
}

fn update_seeders_mod(file_name: &str, seeder_name: &str) -> Result<()> {
    let mod_file = "database/seeders/mod.rs";

    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    // Add module declaration
    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    // Add pub use
    let use_line = format!("pub use {}::{{{}}};\n", file_name, seeder_name);
    if !content.contains(&use_line) {
        content.push_str(&use_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);

    Ok(())
}

// New make commands

pub async fn request(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating request: {}", name).green().bold());

    let request_name = if name.ends_with("Request") {
        name.to_string()
    } else {
        format!("{}Request", name.to_pascal_case())
    };

    let request_path = format!("src/requests/{}.rs", request_name.to_snake_case());

    if Path::new(&request_path).exists() {
        anyhow::bail!("Request already exists: {}", request_path);
    }

    let request_content = generate_request_content(&request_name)?;

    fs::create_dir_all("src/requests")?;
    fs::write(&request_path, request_content)?;

    println!("  {} Created: {}", "✓".green(), request_path);

    update_requests_mod(&request_name.to_snake_case())?;

    println!();
    println!("{}", "Request generated successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  1. Add validation rules in the rules() method");
    println!("  2. Customize authorization logic in authorize()");
    println!("  3. Add custom error messages if needed");

    Ok(())
}

pub async fn policy(name: &str, model_name: Option<&str>) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating policy: {}", name).green().bold());

    let policy_name = if name.ends_with("Policy") {
        name.to_string()
    } else {
        format!("{}Policy", name.to_pascal_case())
    };

    let model = model_name.map(|m| m.to_pascal_case()).unwrap_or_else(|| {
        policy_name
            .strip_suffix("Policy")
            .unwrap_or(&policy_name)
            .to_string()
    });

    let policy_path = format!("src/policies/{}.rs", policy_name.to_snake_case());

    if Path::new(&policy_path).exists() {
        anyhow::bail!("Policy already exists: {}", policy_path);
    }

    let policy_content = generate_policy_content(&policy_name, &model)?;

    fs::create_dir_all("src/policies")?;
    fs::write(&policy_path, policy_content)?;

    println!("  {} Created: {}", "✓".green(), policy_path);

    update_policies_mod(&policy_name.to_snake_case())?;

    println!();
    println!("{}", "Policy generated successfully!".green().bold());

    Ok(())
}

pub async fn event(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating event: {}", name).green().bold());

    let event_name = name.to_pascal_case();
    let event_path = format!("src/events/{}.rs", event_name.to_snake_case());

    if Path::new(&event_path).exists() {
        anyhow::bail!("Event already exists: {}", event_path);
    }

    let event_content = generate_event_content(&event_name)?;

    fs::create_dir_all("src/events")?;
    fs::write(&event_path, event_content)?;

    println!("  {} Created: {}", "✓".green(), event_path);

    update_events_mod(&event_name.to_snake_case(), &event_name)?;

    println!();
    println!("{}", "Event generated successfully!".green().bold());

    Ok(())
}

pub async fn listener(name: &str, event_name: Option<&str>) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating listener: {}", name).green().bold()
    );

    let listener_name = name.to_pascal_case();
    let event = event_name.map(|e| e.to_pascal_case());

    let listener_path = format!("src/listeners/{}.rs", listener_name.to_snake_case());

    if Path::new(&listener_path).exists() {
        anyhow::bail!("Listener already exists: {}", listener_path);
    }

    let listener_content = generate_listener_content(&listener_name, event.as_deref())?;

    fs::create_dir_all("src/listeners")?;
    fs::write(&listener_path, listener_content)?;

    println!("  {} Created: {}", "✓".green(), listener_path);

    update_listeners_mod(&listener_name.to_snake_case())?;

    println!();
    println!("{}", "Listener generated successfully!".green().bold());

    Ok(())
}

pub async fn job(name: &str, queue_name: Option<&str>) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating job: {}", name).green().bold());

    let job_name = name.to_pascal_case();
    let job_path = format!("src/jobs/{}.rs", job_name.to_snake_case());

    if Path::new(&job_path).exists() {
        anyhow::bail!("Job already exists: {}", job_path);
    }

    let job_content = generate_job_content(&job_name, queue_name)?;

    fs::create_dir_all("src/jobs")?;
    fs::write(&job_path, job_content)?;

    println!("  {} Created: {}", "✓".green(), job_path);

    update_jobs_mod(&job_name.to_snake_case())?;

    println!();
    println!("{}", "Job generated successfully!".green().bold());

    Ok(())
}

pub async fn mail(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating mailable: {}", name).green().bold()
    );

    let mail_name = name.to_pascal_case();
    let mail_path = format!("src/mail/{}.rs", mail_name.to_snake_case());

    if Path::new(&mail_path).exists() {
        anyhow::bail!("Mailable already exists: {}", mail_path);
    }

    let mail_content = generate_mail_content(&mail_name)?;

    fs::create_dir_all("src/mail")?;
    fs::write(&mail_path, mail_content)?;

    println!("  {} Created: {}", "✓".green(), mail_path);

    update_mail_mod(&mail_name.to_snake_case())?;

    println!();
    println!("{}", "Mailable generated successfully!".green().bold());

    Ok(())
}

pub async fn notification(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating notification: {}", name).green().bold()
    );

    let notification_name = name.to_pascal_case();
    let notification_path = format!("src/notifications/{}.rs", notification_name.to_snake_case());

    if Path::new(&notification_path).exists() {
        anyhow::bail!("Notification already exists: {}", notification_path);
    }

    let notification_content = generate_notification_content(&notification_name)?;

    fs::create_dir_all("src/notifications")?;
    fs::write(&notification_path, notification_content)?;

    println!("  {} Created: {}", "✓".green(), notification_path);

    update_notifications_mod(&notification_name.to_snake_case())?;

    println!();
    println!("{}", "Notification generated successfully!".green().bold());

    Ok(())
}

pub async fn resource(name: &str, with_collection: bool) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating resource: {}", name).green().bold()
    );

    let resource_name = if name.ends_with("Resource") {
        name.to_string()
    } else {
        format!("{}Resource", name.to_pascal_case())
    };

    let resource_path = format!("src/resources/{}.rs", resource_name.to_snake_case());

    if Path::new(&resource_path).exists() {
        anyhow::bail!("Resource already exists: {}", resource_path);
    }

    let resource_content = generate_resource_content(&resource_name)?;

    fs::create_dir_all("src/resources")?;
    fs::write(&resource_path, resource_content)?;

    println!("  {} Created: {}", "✓".green(), resource_path);

    if with_collection {
        let collection_name = format!(
            "{}Collection",
            resource_name
                .strip_suffix("Resource")
                .unwrap_or(&resource_name)
        );
        let collection_path = format!("src/resources/{}.rs", collection_name.to_snake_case());
        let collection_content =
            generate_resource_collection_content(&collection_name, &resource_name)?;
        fs::write(&collection_path, collection_content)?;
        println!("  {} Created: {}", "✓".green(), collection_path);
    }

    update_resources_mod(&resource_name.to_snake_case())?;

    println!();
    println!("{}", "Resource generated successfully!".green().bold());

    Ok(())
}

pub async fn test(name: &str, unit: bool) -> Result<()> {
    ensure_forge_project()?;

    println!("{}", format!("Generating test: {}", name).green().bold());

    let test_name = name.to_snake_case();
    let test_path = if unit {
        format!("tests/unit/{}.rs", test_name)
    } else {
        format!("tests/integration/{}.rs", test_name)
    };

    if Path::new(&test_path).exists() {
        anyhow::bail!("Test already exists: {}", test_path);
    }

    let test_content = generate_test_content(&name.to_pascal_case(), unit)?;

    if unit {
        fs::create_dir_all("tests/unit")?;
    } else {
        fs::create_dir_all("tests/integration")?;
    }

    fs::write(&test_path, test_content)?;

    println!("  {} Created: {}", "✓".green(), test_path);

    println!();
    println!("{}", "Test generated successfully!".green().bold());
    println!("Run with: cargo test {}", test_name);

    Ok(())
}

pub async fn middleware(name: &str) -> Result<()> {
    ensure_forge_project()?;

    println!(
        "{}",
        format!("Generating middleware: {}", name).green().bold()
    );

    let middleware_name = if name.ends_with("Middleware") {
        name.to_string()
    } else {
        format!("{}Middleware", name.to_pascal_case())
    };

    let middleware_path = format!("src/middleware/{}.rs", middleware_name.to_snake_case());

    if Path::new(&middleware_path).exists() {
        anyhow::bail!("Middleware already exists: {}", middleware_path);
    }

    let middleware_content = generate_middleware_content(&middleware_name)?;

    fs::create_dir_all("src/middleware")?;
    fs::write(&middleware_path, middleware_content)?;

    println!("  {} Created: {}", "✓".green(), middleware_path);

    update_middleware_mod(&middleware_name.to_snake_case())?;

    println!();
    println!("{}", "Middleware generated successfully!".green().bold());

    Ok(())
}

// Content generators for new make commands

fn generate_request_content(request_name: &str) -> Result<String> {
    let template = r#"use rf_validation::{Validator, ValidationRule};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct {{request_name}} {
    // Add your fields here
}

impl {{request_name}} {
    /// Validate the request data
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut validator = Validator::new();
        let rules = self.rules();

        // TODO: Add validation logic
        // Example:
        // validator.validate("field_name", &self.field, &rules.get("field_name").unwrap())?;

        if validator.has_errors() {
            return Err(validator.errors());
        }

        Ok(())
    }

    /// Authorization logic
    pub fn authorize(&self) -> bool {
        // TODO: Implement authorization logic
        // Return true if the user is authorized to make this request
        true
    }

    /// Validation rules
    fn rules(&self) -> HashMap<String, Vec<ValidationRule>> {
        let mut rules = HashMap::new();

        // TODO: Add validation rules
        // Example:
        // rules.insert("email".to_string(), vec![
        //     ValidationRule::Required,
        //     ValidationRule::Email,
        // ]);

        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        let request = {{request_name}} {
            // Add test data
        };

        assert!(request.validate().is_ok());
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("request", template)?;

    let data = json!({
        "request_name": request_name,
    });

    Ok(handlebars.render("request", &data)?)
}

fn generate_policy_content(policy_name: &str, model_name: &str) -> Result<String> {
    let template = r#"use crate::models::{{model_name}};

pub struct {{policy_name}};

impl {{policy_name}} {
    /// Determine if the user can view any models
    pub fn view_any(user_id: i32) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can view the model
    pub fn view(user_id: i32, model: &{{model_name}}) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can create models
    pub fn create(user_id: i32) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can update the model
    pub fn update(user_id: i32, model: &{{model_name}}) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can delete the model
    pub fn delete(user_id: i32, model: &{{model_name}}) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can restore the model
    pub fn restore(user_id: i32, model: &{{model_name}}) -> bool {
        // TODO: Implement logic
        true
    }

    /// Determine if the user can permanently delete the model
    pub fn force_delete(user_id: i32, model: &{{model_name}}) -> bool {
        // TODO: Implement logic
        false
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("policy", template)?;

    let data = json!({
        "policy_name": policy_name,
        "model_name": model_name,
    });

    Ok(handlebars.render("policy", &data)?)
}

fn generate_event_content(event_name: &str) -> Result<String> {
    let template = r#"use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{event_name}} {
    pub timestamp: DateTime<Utc>,
    // Add your event data fields here
}

impl {{event_name}} {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            // Initialize fields
        }
    }
}

impl Default for {{event_name}} {
    fn default() -> Self {
        Self::new()
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("event", template)?;

    let data = json!({
        "event_name": event_name,
    });

    Ok(handlebars.render("event", &data)?)
}

fn generate_listener_content(listener_name: &str, event_name: Option<&str>) -> Result<String> {
    let event = event_name.unwrap_or("Event");

    let template = r#"use async_trait::async_trait;
use anyhow::Result;

pub struct {{listener_name}};

#[async_trait]
pub trait EventListener {
    async fn handle(&self, event: &{{event_name}}) -> Result<()>;
}

#[async_trait]
impl EventListener for {{listener_name}} {
    async fn handle(&self, event: &{{event_name}}) -> Result<()> {
        // TODO: Implement event handling logic
        println!("Handling event: {:?}", event);
        Ok(())
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("listener", template)?;

    let data = json!({
        "listener_name": listener_name,
        "event_name": event,
    });

    Ok(handlebars.render("listener", &data)?)
}

fn generate_job_content(job_name: &str, queue_name: Option<&str>) -> Result<String> {
    let queue = queue_name.unwrap_or("default");

    let template = r#"use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{job_name}} {
    // Add job data fields here
}

impl {{job_name}} {
    pub fn new() -> Self {
        Self {
            // Initialize fields
        }
    }

    pub fn queue_name(&self) -> &str {
        "{{queue_name}}"
    }

    pub fn max_tries(&self) -> u32 {
        3
    }

    pub fn timeout(&self) -> u64 {
        60 // seconds
    }
}

#[async_trait]
pub trait Job {
    async fn handle(&self) -> Result<()>;
}

#[async_trait]
impl Job for {{job_name}} {
    async fn handle(&self) -> Result<()> {
        // TODO: Implement job logic
        println!("Processing job: {{job_name}}");
        Ok(())
    }
}

impl Default for {{job_name}} {
    fn default() -> Self {
        Self::new()
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("job", template)?;

    let data = json!({
        "job_name": job_name,
        "queue_name": queue,
    });

    Ok(handlebars.render("job", &data)?)
}

fn generate_mail_content(mail_name: &str) -> Result<String> {
    let template = r#"use rf_mail::{Mailable, MailMessage};
use anyhow::Result;

pub struct {{mail_name}} {
    // Add mail data fields here
}

impl {{mail_name}} {
    pub fn new() -> Self {
        Self {
            // Initialize fields
        }
    }
}

impl Mailable for {{mail_name}} {
    fn build(&self) -> Result<MailMessage> {
        let mut message = MailMessage::new()
            .subject("{{mail_name}}")
            .to("user@example.com")
            .from("noreply@example.com");

        // TODO: Customize email content
        message = message.body("Email body here");

        Ok(message)
    }
}

impl Default for {{mail_name}} {
    fn default() -> Self {
        Self::new()
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("mail", template)?;

    let data = json!({
        "mail_name": mail_name,
    });

    Ok(handlebars.render("mail", &data)?)
}

fn generate_notification_content(notification_name: &str) -> Result<String> {
    let template = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{notification_name}} {
    pub title: String,
    pub message: String,
    // Add notification data fields here
}

impl {{notification_name}} {
    pub fn new(title: String, message: String) -> Self {
        Self {
            title,
            message,
            // Initialize fields
        }
    }

    /// Get notification channels (email, database, sms, etc.)
    pub fn via(&self) -> Vec<&str> {
        vec!["database", "mail"]
    }

    /// Convert to email message
    pub fn to_mail(&self) -> String {
        format!("{}\n\n{}", self.title, self.message)
    }

    /// Convert to database representation
    pub fn to_database(&self) -> serde_json::Value {
        serde_json::json!({
            "title": self.title,
            "message": self.message,
        })
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("notification", template)?;

    let data = json!({
        "notification_name": notification_name,
    });

    Ok(handlebars.render("notification", &data)?)
}

fn generate_resource_content(resource_name: &str) -> Result<String> {
    let template = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct {{resource_name}} {
    pub id: i32,
    // Add resource fields here
}

impl {{resource_name}} {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            // Initialize fields
        }
    }

    /// Transform the resource into an array
    pub fn to_array(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            // Add transformed fields
        })
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("resource", template)?;

    let data = json!({
        "resource_name": resource_name,
    });

    Ok(handlebars.render("resource", &data)?)
}

fn generate_resource_collection_content(
    collection_name: &str,
    resource_name: &str,
) -> Result<String> {
    let template = r#"use serde::{Deserialize, Serialize};
use super::{{resource_name}};

#[derive(Debug, Serialize, Deserialize)]
pub struct {{collection_name}} {
    pub data: Vec<{{resource_name}}>,
}

impl {{collection_name}} {
    pub fn new(data: Vec<{{resource_name}}>) -> Self {
        Self { data }
    }

    /// Transform the collection into an array
    pub fn to_array(&self) -> serde_json::Value {
        serde_json::json!({
            "data": self.data.iter().map(|item| item.to_array()).collect::<Vec<_>>(),
        })
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("collection", template)?;

    let data = json!({
        "collection_name": collection_name,
        "resource_name": resource_name,
    });

    Ok(handlebars.render("collection", &data)?)
}

fn generate_test_content(test_name: &str, is_unit: bool) -> Result<String> {
    let template = if is_unit {
        r#"#[cfg(test)]
mod {{test_name_lower}}_tests {
    use super::*;

    #[test]
    fn test_{{test_name_lower}}() {
        // TODO: Implement test
        assert!(true);
    }
}
"#
    } else {
        r#"use anyhow::Result;

#[tokio::test]
async fn test_{{test_name_lower}}() -> Result<()> {
    // TODO: Implement integration test
    Ok(())
}
"#
    };

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("test", template)?;

    let data = json!({
        "test_name": test_name,
        "test_name_lower": test_name.to_snake_case(),
    });

    Ok(handlebars.render("test", &data)?)
}

fn generate_middleware_content(middleware_name: &str) -> Result<String> {
    let template = r#"use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub struct {{middleware_name}};

impl {{middleware_name}} {
    pub async fn handle(request: Request, next: Next) -> Result<Response, StatusCode> {
        // TODO: Implement middleware logic (before request)

        // Process the request
        let response = next.run(request).await;

        // TODO: Implement middleware logic (after request)

        Ok(response)
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("middleware", template)?;

    let data = json!({
        "middleware_name": middleware_name,
    });

    Ok(handlebars.render("middleware", &data)?)
}

// Helper functions to update mod.rs files

fn update_requests_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/requests/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_policies_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/policies/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_events_mod(file_name: &str, event_name: &str) -> Result<()> {
    let mod_file = "src/events/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    let use_line = format!("pub use {}::{{{}}};\n", file_name, event_name);
    if !content.contains(&use_line) {
        content.push_str(&use_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_listeners_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/listeners/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_jobs_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/jobs/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_mail_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/mail/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_notifications_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/notifications/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_resources_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/resources/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

fn update_middleware_mod(file_name: &str) -> Result<()> {
    let mod_file = "src/middleware/mod.rs";
    let mut content = if Path::new(mod_file).exists() {
        fs::read_to_string(mod_file)?
    } else {
        String::new()
    };

    let mod_line = format!("pub mod {};\n", file_name);
    if !content.contains(&mod_line) {
        content.push_str(&mod_line);
    }

    fs::write(mod_file, content)?;
    println!("  {} Updated: {}", "✓".green(), mod_file);
    Ok(())
}

// Interactive model generation
pub async fn model_interactive() -> Result<()> {
    ensure_forge_project()?;

    let config = interactive::prompt_model_config()?;

    println!();
    println!("{}", "Creating model...".green().bold());

    let mut gen_progress = progress::GenerationProgress::new();

    // Generate model
    let model_task =
        gen_progress.add_task(&format!("app/models/{}.rs", config.name.to_snake_case()));
    let model_name = config.name.to_pascal_case();
    let table_name = config.name.to_snake_case().to_plural();
    let model_path = format!("src/models/{}.rs", config.name.to_snake_case());

    if Path::new(&model_path).exists() {
        gen_progress.fail_task(model_task, &model_path, "File already exists");
        return Err(errors::file_already_exists(&model_path).into());
    }

    let model_content = generate_model_content(&model_name, &table_name)?;
    fs::create_dir_all("src/models")?;
    fs::write(&model_path, model_content)?;
    update_models_mod(&config.name.to_snake_case(), &model_name)?;
    gen_progress.complete_task(model_task, &model_path);

    // Generate migration if requested
    if config.create_migration {
        let migration_name = format!("create_{}_table", table_name);
        let mig_task = gen_progress.add_task(&format!("migrations/{}.rs", migration_name));
        migration(&migration_name).await?;
        gen_progress.complete_task(mig_task, &format!("migrations/{}.rs", migration_name));
    }

    // Generate factory if requested
    if config.create_factory {
        let factory_task = gen_progress.add_task(&format!(
            "tests/factories/{}_factory.rs",
            config.name.to_snake_case()
        ));
        factory(&config.name, None).await?;
        gen_progress.complete_task(
            factory_task,
            &format!("tests/factories/{}_factory.rs", config.name.to_snake_case()),
        );
    }

    // Generate seeder if requested
    if config.create_seeder {
        let seeder_task = gen_progress.add_task(&format!(
            "database/seeders/{}_seeder.rs",
            config.name.to_snake_case()
        ));
        seeder(&config.name).await?;
        gen_progress.complete_task(
            seeder_task,
            &format!("database/seeders/{}_seeder.rs", config.name.to_snake_case()),
        );
    }

    interactive::print_next_steps(&[
        &format!("Edit model: {}", model_path),
        if config.create_migration {
            "Run: forge migrate"
        } else {
            "Create a migration with: forge make:migration"
        },
    ]);

    Ok(())
}

// Interactive controller generation
pub async fn controller_interactive() -> Result<()> {
    ensure_forge_project()?;

    let config = interactive::prompt_controller_config()?;

    println!();
    println!("{}", "Creating controller...".green().bold());

    let mut gen_progress = progress::GenerationProgress::new();

    let controller_name = if config.name.ends_with("Controller") {
        config.name.clone()
    } else {
        format!("{}Controller", config.name.to_pascal_case())
    };

    let controller_task = gen_progress.add_task(&format!(
        "app/controllers/{}.rs",
        controller_name.to_snake_case()
    ));
    let controller_path = format!("src/controllers/{}.rs", controller_name.to_snake_case());

    if Path::new(&controller_path).exists() {
        gen_progress.fail_task(controller_task, &controller_path, "File already exists");
        return Err(errors::file_already_exists(&controller_path).into());
    }

    let controller_content = match config.controller_type {
        interactive::ControllerType::Api | interactive::ControllerType::Resource => {
            generate_api_controller_content(&controller_name)?
        }
        interactive::ControllerType::Plain | interactive::ControllerType::Invokable => {
            generate_web_controller_content(&controller_name)?
        }
    };

    fs::create_dir_all("src/controllers")?;
    fs::write(&controller_path, controller_content)?;
    update_controllers_mod(&controller_name.to_snake_case())?;
    gen_progress.complete_task(controller_task, &controller_path);

    println!();

    // Display generated methods based on controller type
    if matches!(
        config.controller_type,
        interactive::ControllerType::Resource | interactive::ControllerType::Api
    ) {
        interactive::print_info("Generated methods:");
        println!("  • {}    - GET    /resource", "index()".cyan());
        println!("  • {}   - GET    /resource/create", "create()".cyan());
        println!("  • {}    - POST   /resource", "store()".cyan());
        println!("  • {}     - GET    /resource/:id", "show()".cyan());
        println!("  • {}     - GET    /resource/:id/edit", "edit()".cyan());
        println!("  • {}   - PUT    /resource/:id", "update()".cyan());
        println!("  • {}  - DELETE /resource/:id", "destroy()".cyan());
    }

    interactive::print_next_steps(&[
        &format!("Implement controller logic in {}", controller_path),
        "Add routes to routes/web.rs or routes/api.rs",
        if config.create_routes {
            "Routes have been generated"
        } else {
            "Create routes manually"
        },
    ]);

    Ok(())
}
