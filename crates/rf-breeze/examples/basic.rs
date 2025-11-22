//! Basic example of using rf-breeze to scaffold authentication
//!
//! Run with: cargo run --example basic

use rf_breeze::{BreezeScaffold, InstallOptions};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 rf-breeze - Authentication Scaffolding Example\n");

    // Create a temporary directory for the example
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    println!("📁 Using temporary directory: {}\n", project_path.display());

    // Create the Breeze scaffold
    let breeze = BreezeScaffold::new(project_path)?;

    println!("✨ Installing authentication scaffolding...\n");

    // Install with full options
    breeze
        .install(&InstallOptions {
            with_api: true,
            with_email_verification: true,
            with_password_reset: true,
            output_dir: None,
        })
        .await?;

    println!("✅ Installation complete!\n");

    // Show what was created
    println!("📦 Generated Structure:\n");

    // List views
    println!("Views:");
    let views_dir = project_path.join("resources/views");
    if views_dir.exists() {
        for entry in std::fs::read_dir(views_dir.join("auth"))? {
            let entry = entry?;
            println!(
                "  - resources/views/auth/{}",
                entry.file_name().to_string_lossy()
            );
        }
    }

    // List controllers
    println!("\nControllers:");
    let controllers_dir = project_path.join("src/controllers/auth");
    if controllers_dir.exists() {
        for entry in std::fs::read_dir(&controllers_dir)? {
            let entry = entry?;
            println!(
                "  - src/controllers/auth/{}",
                entry.file_name().to_string_lossy()
            );
        }
    }

    // List routes
    println!("\nRoutes:");
    let routes_dir = project_path.join("src/routes");
    if routes_dir.exists() {
        for entry in std::fs::read_dir(&routes_dir)? {
            let entry = entry?;
            println!("  - src/routes/{}", entry.file_name().to_string_lossy());
        }
    }

    // List middleware
    println!("\nMiddleware:");
    let middleware_dir = project_path.join("src/middleware");
    if middleware_dir.exists() {
        for entry in std::fs::read_dir(&middleware_dir)? {
            let entry = entry?;
            println!("  - src/middleware/{}", entry.file_name().to_string_lossy());
        }
    }

    println!("\n🎉 Authentication scaffolding successfully installed!");
    println!("\n💡 Next steps:");
    println!("   1. Review generated views in resources/views/auth/");
    println!("   2. Implement database logic in src/controllers/auth/");
    println!("   3. Integrate routes in your application");
    println!("   4. Configure rf-auth and rf-blade");
    println!("   5. Test the authentication flow");

    Ok(())
}
