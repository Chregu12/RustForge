//! Example of selectively installing authentication components
//!
//! Run with: cargo run --example selective

use rf_breeze::BreezeScaffold;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 rf-breeze - Selective Installation Example\n");

    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let project_path = temp_dir.path();

    println!("📁 Using temporary directory: {}\n", project_path.display());

    // Create the Breeze scaffold
    let breeze = BreezeScaffold::new(project_path)?;

    // Install components one by one
    println!("📄 Installing views...");
    breeze.install_views().await?;
    println!("   ✅ Views installed");

    println!("\n🎮 Installing controllers...");
    breeze.install_controllers().await?;
    println!("   ✅ Controllers installed");

    println!("\n🛣️  Installing routes...");
    breeze.install_routes().await?;
    println!("   ✅ Routes installed");

    println!("\n🛡️  Installing middleware...");
    breeze.install_middleware().await?;
    println!("   ✅ Middleware installed");

    println!("\n✨ All components installed successfully!");

    // Verify installation
    let views_exist = project_path.join("resources/views/auth/login.blade.html").exists();
    let controllers_exist = project_path.join("src/controllers/auth/login.rs").exists();
    let routes_exist = project_path.join("src/routes/auth.rs").exists();
    let middleware_exist = project_path.join("src/middleware/auth.rs").exists();

    println!("\n📊 Verification:");
    println!("   Views: {}", if views_exist { "✅" } else { "❌" });
    println!("   Controllers: {}", if controllers_exist { "✅" } else { "❌" });
    println!("   Routes: {}", if routes_exist { "✅" } else { "❌" });
    println!("   Middleware: {}", if middleware_exist { "✅" } else { "❌" });

    if views_exist && controllers_exist && routes_exist && middleware_exist {
        println!("\n🎉 All components verified!");
    }

    Ok(())
}
