//! View Composers Example
//!
//! Demonstrates how to use view composers to share data across multiple views

use rf_views::prelude::*;
use serde_json::json;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 View Composers Example\n");

    // Create a local composer registry (not using global)
    let registry = Arc::new(ComposerRegistry::new());

    // 1. Global composer - applies to all views
    println!("1️⃣  Registering global composer (applies to all views)...");
    registry.composer_fn("*", |view_name, context| {
        context.insert("app_name", "MyApp");
        context.insert("app_version", "1.0.0");
        println!("   ✓ Global composer applied to: {}", view_name);
        Ok(())
    })?;

    // 2. Specific pattern composer
    println!("\n2️⃣  Registering pattern-specific composers...");

    registry.composer_fn("posts.*", |view_name, context| {
        context.insert("section", "posts");
        context.insert("categories", json!(["Tech", "Design", "Business"]));
        println!("   ✓ Posts composer applied to: {}", view_name);
        Ok(())
    })?;

    registry.composer_fn("admin.*", |view_name, context| {
        context.insert("section", "admin");
        context.insert("admin_menu", json!([
            {"label": "Dashboard", "url": "/admin"},
            {"label": "Users", "url": "/admin/users"},
            {"label": "Settings", "url": "/admin/settings"}
        ]));
        println!("   ✓ Admin composer applied to: {}", view_name);
        Ok(())
    })?;

    // 3. View creator - runs before composers
    println!("\n3️⃣  Registering view creators (run before composers)...");

    registry.creator_fn("posts.index", |view_name, context| {
        context.insert("initial_data", "Creator data");
        println!("   ✓ Creator ran for: {}", view_name);
        Ok(())
    })?;

    // 4. Test composing different views
    println!("\n4️⃣  Testing view composition...\n");

    // Test posts.index view
    println!("   Composing 'posts.index' view:");
    let mut posts_context = Context::new();
    registry.compose("posts.index", &mut posts_context)?;

    println!("   Data in context:");
    println!("     - app_name: {:?}", posts_context.get("app_name"));
    println!("     - section: {:?}", posts_context.get("section"));
    println!("     - categories: {:?}", posts_context.get("categories"));
    println!("     - initial_data: {:?}", posts_context.get("initial_data"));

    // Test admin.users view
    println!("\n   Composing 'admin.users' view:");
    let mut admin_context = Context::new();
    registry.compose("admin.users", &mut admin_context)?;

    println!("   Data in context:");
    println!("     - app_name: {:?}", admin_context.get("app_name"));
    println!("     - section: {:?}", admin_context.get("section"));
    println!("     - admin_menu: {:?}", admin_context.get("admin_menu"));

    // Test a view with no specific composer
    println!("\n   Composing 'welcome' view (only global composer):");
    let mut welcome_context = Context::new();
    registry.compose("welcome", &mut welcome_context)?;

    println!("   Data in context:");
    println!("     - app_name: {:?}", welcome_context.get("app_name"));
    println!("     - section: {:?}", welcome_context.get("section"));

    // 5. Using global composers
    println!("\n5️⃣  Using global composers...");

    // Register a global composer
    composers::composer_fn("profile.*", |_, context| {
        context.insert("user_section", "profile");
        Ok(())
    })?;

    let mut profile_context = Context::new();
    composers::global().compose("profile.edit", &mut profile_context)?;

    println!("   Global composer applied:");
    println!("     - user_section: {:?}", profile_context.get("user_section"));

    // 6. Composer statistics
    println!("\n6️⃣  Composer statistics:");
    println!("   - Composers registered: {}", registry.composer_count());
    println!("   - Creators registered: {}", registry.creator_count());

    // 7. Custom ViewComposer trait implementation
    println!("\n7️⃣  Custom ViewComposer implementation...");

    struct UserDataComposer {
        user_id: i32,
    }

    impl ViewComposer for UserDataComposer {
        fn compose(&self, _view_name: &str, context: &mut Context) -> ViewResult<()> {
            context.insert("current_user_id", self.user_id);
            context.insert("is_authenticated", true);
            Ok(())
        }

        fn registered(&self) {
            println!("   ✓ UserDataComposer registered for user {}", self.user_id);
        }
    }

    registry.composer("user.*", UserDataComposer { user_id: 123 })?;

    let mut user_context = Context::new();
    registry.compose("user.dashboard", &mut user_context)?;

    println!("   User data added to context:");
    println!("     - current_user_id: {:?}", user_context.get("current_user_id"));
    println!("     - is_authenticated: {:?}", user_context.get("is_authenticated"));

    println!("\n✅ All view composer examples completed successfully!");

    Ok(())
}
