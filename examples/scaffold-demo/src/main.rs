//! rf-scaffold demonstration
//!
//! This example demonstrates the code generation capabilities of rf-scaffold.

use rf_scaffold::{ScaffoldEngine, ModelOptions};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== RustForge Scaffold Demo ===\n");

    // Create a temporary directory for demo
    let demo_dir = PathBuf::from("/tmp/rf-scaffold-demo");
    std::fs::create_dir_all(&demo_dir)?;

    let scaffold = ScaffoldEngine::new(&demo_dir)?;

    // 1. Generate a Model
    println!("1. Generating User model with fields...");
    let user_model = scaffold.generate_model("User", &ModelOptions {
        fields: vec![
            ("name", "String"),
            ("email", "String"),
            ("age", "i32"),
            ("active", "bool"),
        ],
        with_migration: true,
        with_factory: false,
    }).await?;
    println!("   ✓ Generated: {}", user_model.display());

    // 2. Generate a Controller
    println!("\n2. Generating UserController...");
    let controller = scaffold.generate_controller("UserController", false).await?;
    println!("   ✓ Generated: {}", controller.display());

    // 3. Generate a Resource Controller
    println!("\n3. Generating PostController (resource)...");
    let resource_controller = scaffold.generate_controller("PostController", true).await?;
    println!("   ✓ Generated: {}", resource_controller.display());

    // 4. Generate a Service
    println!("\n4. Generating AuthenticationService...");
    let service = scaffold.generate_service("AuthenticationService").await?;
    println!("   ✓ Generated: {}", service.display());

    // 5. Generate a Migration
    println!("\n5. Generating migration...");
    let migration = scaffold.generate_migration("add_verified_to_users").await?;
    println!("   ✓ Generated: {}", migration.display());

    // 6. Register and use custom template
    println!("\n6. Demonstrating custom templates...");
    scaffold.register_template(
        "custom-struct",
        r#"// Custom generated struct
pub struct {{name}} {
    pub id: u64,
    pub data: String,
}

impl {{name}} {
    pub fn new(data: String) -> Self {
        Self { id: 0, data }
    }
}
"#
    ).await?;
    println!("   ✓ Registered custom template");

    // 7. Demonstrate naming conventions
    println!("\n7. Naming convention utilities:");
    let naming = scaffold.naming();
    println!("   PascalCase: user_controller -> {}", naming.to_pascal_case("user_controller"));
    println!("   snake_case: UserController -> {}", naming.to_snake_case("UserController"));
    println!("   kebab-case: UserService -> {}", naming.to_kebab_case("UserService"));
    println!("   Pluralize: user -> {}", naming.pluralize("user"));
    println!("   Singularize: users -> {}", naming.singularize("users"));
    println!("   Extract base: UserController -> {}", naming.extract_base("UserController"));

    println!("\n=== Demo Complete ===");
    println!("Generated files are in: {}", demo_dir.display());
    println!("\nYou can inspect the generated code to see the quality and structure.");

    Ok(())
}
