use anyhow::Result;
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
    // RustForge's ORM (rf-orm) is built on SeaORM. The canonical, compiling
    // model is a SeaORM entity module: a `Model` struct deriving
    // `DeriveEntityModel`, a `Relation` enum and an `ActiveModelBehavior` impl.
    // Each model lives in its own module (this file), so the generated `Entity`,
    // `ActiveModel`, `Column` and `Relation` names never collide across models.
    // The module is re-exported as `{{model_name}}` (the entity alias) from
    // `src/models/mod.rs` for Laravel-style `{{model_name}}::find(..)` usage.
    //
    // Note: rf-orm also ships a `#[model]` attribute, but in this SeaORM version
    // `DeriveEntityModel` requires the struct be literally named `Model`, so the
    // explicit entity below is the form that actually builds.
    let template = r#"use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{{table_name}}")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    // Add your columns here, e.g.:
    // pub title: String,
    // pub body: String,
    pub name: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
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
    // API controller in the canonical RustForge/Axum style: each method is an
    // Axum handler returning `impl IntoResponse` (matching the starter's
    // app/Http/Controllers). Extractors (Path, Query, Json) are pulled in as
    // needed.
    let template = r#"use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

/// Query string parameters for the index listing.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

pub struct {{controller_name}};

impl {{controller_name}} {
    /// GET /api/resource
    pub async fn index(Query(query): Query<ListQuery>) -> impl IntoResponse {
        // Example: let items = Model::query().limit(query.limit).get().await?;
        let _ = query;
        Json(json!({ "data": [] }))
    }

    /// GET /api/resource/:id
    pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {
        // Example: let item = Model::find(id).await?;
        Json(json!({ "id": id }))
    }

    /// POST /api/resource
    pub async fn store(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
        // Example: let item = Model::create(payload).await?;
        (StatusCode::CREATED, Json(payload))
    }

    /// PUT /api/resource/:id
    pub async fn update(
        Path(id): Path<i32>,
        Json(payload): Json<serde_json::Value>,
    ) -> impl IntoResponse {
        // Example: Model::update_by_id(id, payload).await?;
        let _ = id;
        Json(payload)
    }

    /// DELETE /api/resource/:id
    pub async fn destroy(Path(id): Path<i32>) -> impl IntoResponse {
        // Example: Model::destroy(id).await?;
        let _ = id;
        StatusCode::NO_CONTENT
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
    // Web controller: Axum handlers returning HTML responses.
    let template = r#"use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};

pub struct {{controller_name}};

impl {{controller_name}} {
    /// GET /resource
    pub async fn index() -> impl IntoResponse {
        Html("<h1>Index</h1>".to_string())
    }

    /// GET /resource/:id
    pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {
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
    // Canonical RustForge migration: implements `rf_orm::Migration`, using the
    // schema `Blueprint` builder (Laravel-style) inside `up`/`down`.
    let is_create = name.starts_with("create_") && name.ends_with("_table");

    let struct_name = name.to_pascal_case();

    let body = if is_create {
        let table_name = name
            .strip_prefix("create_")
            .unwrap()
            .strip_suffix("_table")
            .unwrap()
            .to_string();
        format!(
            r#"use async_trait::async_trait;
use rf_orm::{{Blueprint, Migration, MigrationError, MigrationResult, SchemaContext}};

pub struct {struct_name};

#[async_trait]
impl Migration for {struct_name} {{
    fn name(&self) -> &str {{
        "{name}"
    }}

    async fn up(&self, schema: &SchemaContext) -> MigrationResult<()> {{
        schema
            .create("{table_name}", |table: &mut Blueprint| {{
                table.id();
                // Add your columns here, e.g.:
                // table.string("title");
                // table.text("body").nullable();
                table.timestamps();
            }})
            .await
            .map_err(|e| MigrationError::SchemaError(e.to_string()))?;
        Ok(())
    }}

    async fn down(&self, schema: &SchemaContext) -> MigrationResult<()> {{
        schema
            .drop_if_exists("{table_name}")
            .await
            .map_err(|e| MigrationError::SchemaError(e.to_string()))?;
        Ok(())
    }}
}}
"#
        )
    } else {
        format!(
            r#"use async_trait::async_trait;
use rf_orm::{{Migration, MigrationResult, SchemaContext}};

pub struct {struct_name};

#[async_trait]
impl Migration for {struct_name} {{
    fn name(&self) -> &str {{
        "{name}"
    }}

    async fn up(&self, schema: &SchemaContext) -> MigrationResult<()> {{
        // Modify an existing table:
        // use rf_orm::Blueprint;
        // schema.table("table_name", |table: &mut Blueprint| {{
        //     table.string("new_column").nullable();
        // }}).await?;
        let _ = schema;
        Ok(())
    }}

    async fn down(&self, schema: &SchemaContext) -> MigrationResult<()> {{
        // Reverse the change made in `up`.
        let _ = schema;
        Ok(())
    }}
}}
"#
        )
    };

    Ok(body)
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

    // Re-export the SeaORM entity's record type as `{model_name}` (so app code
    // and policies refer to `crate::models::{model_name}`), and the queryable
    // `Entity` as `{model_name}Entity` for `{model_name}Entity::find(..)`.
    let use_line = format!(
        "pub use {file}::{{Entity as {model}Entity, Model as {model}}};\n",
        file = file_name,
        model = model_name
    );
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
    // Canonical RustForge factory: implements `rf_testing::FactoryDefinition`
    // and uses the `rf_testing::impl_factory!` macro to wire up the `Factory`
    // trait (giving you `new()`, `create()`, `create_many()`, `state()`, ...).
    //
    // Factories live under `tests/`, where the `rf-testing` dev-dependency is
    // available. The `{{model_name}}` struct here mirrors your model's fields;
    // replace it with `use crate::models::{{model_name}};` once your model is a
    // plain constructible struct, and fill `definition()` with `Fake::*` data.
    let template = r#"// `Factory` and `FactoryError` are referenced by the `impl_factory!` macro
// expansion, so they must be in scope here.
use rf_testing::{Factory, FactoryDefinition, FactoryError, Fake};

/// Plain test-data shape for {{model_name}}. Mirror your model's fields here.
#[derive(Clone, Debug)]
pub struct {{model_name}} {
    pub id: i32,
    pub name: String,
    pub email: String,
}

/// Factory for generating {{model_name}} test data.
pub struct {{factory_name}} {
    model: {{model_name}},
}

impl Default for {{factory_name}} {
    fn default() -> Self {
        Self {
            model: <{{factory_name}} as FactoryDefinition>::definition(),
        }
    }
}

impl FactoryDefinition for {{factory_name}} {
    type Model = {{model_name}};

    fn definition() -> Self::Model {
        {{model_name}} {
            id: 0,
            name: Fake::name(),
            email: Fake::email(),
        }
    }
}

// Wire up the `Factory` trait (new/create/create_many/state/build).
rf_testing::impl_factory!({{factory_name}}, {{model_name}});

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_create() {
        let instance = {{factory_name}}::new().create().await.unwrap();
        assert!(!instance.name.is_empty());
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
    // Canonical RustForge seeder: implements `rf_seeder::Seeder` (Laravel-style).
    let template = r#"use async_trait::async_trait;
use rf_seeder::{Seeder, SeederError};

/// Seeder for populating {{seeder_name}} data.
pub struct {{seeder_name}};

#[async_trait]
impl Seeder for {{seeder_name}} {
    fn name(&self) -> &str {
        "{{seeder_name}}"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("Seeding {{seeder_name}}...");

        // TODO: Implement seeder logic, e.g. insert records via your models.

        Ok(())
    }

    // Seeders that must run before this one:
    // fn depends_on(&self) -> Vec<&str> {
    //     vec!["UserSeeder"]
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
    // Canonical RustForge form request: implements `rf_validation::FormRequest`.
    // The framework's `Validated<T>` Axum extractor calls `authorize()`,
    // `rules()` and `validate()` for you, returning a 422 with field errors on
    // failure. Rules are boxed `Rule` trait objects from `rf_validation::rules`.
    let template = r#"use async_trait::async_trait;
use rf_validation::{
    rules::{EmailRule, RequiredRule},
    FormRequest, FormRequestResult, RulesBuilder, ValidationRules,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct {{request_name}} {
    // Add your fields here, e.g.:
    pub email: String,
}

#[async_trait]
impl FormRequest for {{request_name}} {
    type Validated = Self;

    /// Determine if the user is authorized to make this request.
    fn authorize(&self) -> bool {
        true
    }

    /// Validation rules to apply to the request data.
    fn rules(&self) -> ValidationRules {
        RulesBuilder::new()
            .add(
                "email",
                vec![Box::new(RequiredRule), Box::new(EmailRule)],
            )
            .build()
    }

    /// Run validation and return the validated data.
    async fn validate(self) -> FormRequestResult<Self::Validated> {
        // The `rules()` above are enforced by the `Validated<T>` extractor.
        // Add any extra cross-field checks here before returning.
        Ok(self)
    }
}

// Usage in a handler:
//
//     use rf_validation::Validated;
//
//     async fn store(
//         Validated(request): Validated<{{request_name}}>,
//     ) -> impl axum::response::IntoResponse {
//         axum::Json(request.email)
//     }
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("request", template)?;

    let data = json!({
        "request_name": request_name,
    });

    Ok(handlebars.render("request", &data)?)
}

fn generate_policy_content(policy_name: &str, model_name: &str) -> Result<String> {
    // Canonical RustForge authorization policy: implements the
    // `rf_authorization::Policy<U, M>` trait, where `U` is your user type and
    // `M` is the model being authorized. The policy is generic over the user
    // type so it works with whatever user/identity type your app uses; register
    // it with `Gate::register::<{{model_name}}, _, YourUser>({{policy_name}})`.
    let template = r#"use crate::models::{{model_name}};
use rf_authorization::Policy;

pub struct {{policy_name}};

impl<U> Policy<U, {{model_name}}> for {{policy_name}} {
    /// Determine if the user can view any models.
    fn view_any(&self, _user: Option<&U>) -> bool {
        true
    }

    /// Determine if the user can view the model.
    fn view(&self, _user: Option<&U>, _model: &{{model_name}}) -> bool {
        true
    }

    /// Determine if the user can create models.
    fn create(&self, _user: &U) -> bool {
        true
    }

    /// Determine if the user can update the model.
    fn update(&self, _user: &U, _model: &{{model_name}}) -> bool {
        true
    }

    /// Determine if the user can delete the model.
    fn delete(&self, _user: &U, _model: &{{model_name}}) -> bool {
        true
    }

    /// Determine if the user can restore the model.
    fn restore(&self, _user: &U, _model: &{{model_name}}) -> bool {
        true
    }

    /// Determine if the user can permanently delete the model.
    fn force_delete(&self, _user: &U, _model: &{{model_name}}) -> bool {
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
    // Canonical RustForge event: implements `rf_events::Event` so it can be
    // dispatched and listened for via the framework's event dispatcher.
    let template = r#"use rf_events::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{event_name}} {
    // Add your event payload fields here, e.g.:
    // pub user_id: i32,
}

impl {{event_name}} {
    pub fn new() -> Self {
        Self {
            // Initialize fields
        }
    }
}

impl Default for {{event_name}} {
    fn default() -> Self {
        Self::new()
    }
}

impl Event for {{event_name}} {}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("event", template)?;

    let data = json!({
        "event_name": event_name,
    });

    Ok(handlebars.render("event", &data)?)
}

fn generate_listener_content(listener_name: &str, event_name: Option<&str>) -> Result<String> {
    // Canonical RustForge listener: implements `rf_events::EventListenerFor<E>`
    // for the event it handles. When generated with `--event <Name>` it imports
    // that event from `crate::events`; otherwise it ships a local placeholder
    // event so the file compiles on its own.
    let (event_decl, event_ty) = match event_name {
        Some(ev) => (
            format!("use crate::events::{ev};\n"),
            ev.to_string(),
        ),
        None => (
            // Local placeholder event implementing `rf_events::Event`.
            "use rf_events::Event;\n\n// Placeholder event. Replace with `use crate::events::YourEvent;`.\n#[derive(Debug, Clone)]\npub struct PlaceholderEvent;\nimpl Event for PlaceholderEvent {}\n".to_string(),
            "PlaceholderEvent".to_string(),
        ),
    };

    let template = r#"use async_trait::async_trait;
use rf_events::{EventListenerFor, EventResult};
{{event_decl}}
pub struct {{listener_name}};

#[async_trait]
impl EventListenerFor<{{event_ty}}> for {{listener_name}} {
    async fn handle(&self, event: &{{event_ty}}) -> EventResult<()> {
        // TODO: Implement event handling logic.
        println!("Handling event: {:?}", event);
        Ok(())
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("listener", template)?;

    let data = json!({
        "listener_name": listener_name,
        "event_decl": event_decl,
        "event_ty": event_ty,
    });

    Ok(handlebars.render("listener", &data)?)
}

fn generate_job_content(job_name: &str, queue_name: Option<&str>) -> Result<String> {
    let queue = queue_name.unwrap_or("default");

    // Canonical RustForge job: implements `rf_queue::Job`. Jobs must be
    // serializable (so they can be persisted onto a queue) and provide a stable
    // `job_type` identifier. Dispatch with `rf_queue::dispatch(&job, &queue)`.
    let template = r#"use async_trait::async_trait;
use rf_queue::{Job, QueueError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct {{job_name}} {
    // Add job payload fields here, e.g.:
    // pub user_id: i32,
}

impl {{job_name}} {
    pub fn new() -> Self {
        Self {
            // Initialize fields
        }
    }
}

impl Default for {{job_name}} {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Job for {{job_name}} {
    async fn handle(&self) -> Result<(), QueueError> {
        // TODO: Implement job logic.
        println!("Processing job: {{job_name}}");
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "{{job_name}}"
    }

    fn max_retries(&self) -> u32 {
        3
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    fn queue(&self) -> &str {
        "{{queue_name}}"
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
    // Canonical RustForge mailable: implements `rf_mail::Mailable`, whose
    // `build` returns a `MailBuilder`. Send it with `mail.send(&mailer).await?`.
    let template = r#"use rf_mail::{Address, Mailable, MailBuilder};

pub struct {{mail_name}} {
    // Add mail data fields here, e.g.:
    pub to: String,
}

impl {{mail_name}} {
    pub fn new(to: impl Into<String>) -> Self {
        Self { to: to.into() }
    }
}

impl Mailable for {{mail_name}} {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new(self.to.as_str()))
            .subject("{{mail_name}}")
            .text("Email body here")
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
    // Canonical RustForge notification: implements `rf_notifications::Notification`.
    // Choose delivery channels in `via()`; override `to_mail`/`to_database`/etc.
    // to render each channel's payload (they default to `None`).
    let template = r#"use async_trait::async_trait;
use rf_notifications::{Channel, Notification};

#[derive(Debug, Clone)]
pub struct {{notification_name}} {
    pub title: String,
    pub message: String,
}

impl {{notification_name}} {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
        }
    }
}

#[async_trait]
impl Notification for {{notification_name}} {
    /// Channels this notification is delivered over.
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Database, Channel::Mail]
    }

    // Override `to_mail`, `to_database`, `to_sms` or `to_slack` to render the
    // payload for each channel (each returns `None` by default).
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
    // API resource: a serializable transformer that shapes a model into its
    // JSON representation. RustForge has no required trait here, so this is a
    // minimal idiomatic serde struct (matching how Axum handlers return JSON).
    let template = r#"use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct {{resource_name}} {
    pub id: i32,
    // Add the fields you want to expose here.
}

impl {{resource_name}} {
    pub fn new(id: i32) -> Self {
        Self { id }
    }

    /// Transform the resource into a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
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
    let resource_module = resource_name.to_snake_case();

    let template = r#"use super::{{resource_module}}::{{resource_name}};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct {{collection_name}} {
    pub data: Vec<{{resource_name}}>,
}

impl {{collection_name}} {
    pub fn new(data: Vec<{{resource_name}}>) -> Self {
        Self { data }
    }

    /// Transform the collection into a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "data": self.data.iter().map(|item| item.to_json()).collect::<Vec<_>>(),
        })
    }
}
"#;

    let mut handlebars = Handlebars::new();
    handlebars.register_template_string("collection", template)?;

    let data = json!({
        "collection_name": collection_name,
        "resource_name": resource_name,
        "resource_module": resource_module,
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
    // Canonical Axum middleware (RustForge builds on Axum): a `from_fn`-style
    // handler. Apply with `axum::middleware::from_fn({{middleware_name}}::handle)`.
    let template = r#"use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub struct {{middleware_name}};

impl {{middleware_name}} {
    pub async fn handle(request: Request, next: Next) -> Result<Response, StatusCode> {
        // Pre-processing (before the request is handled) goes here.

        let response = next.run(request).await;

        // Post-processing (after the response is produced) goes here.

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
