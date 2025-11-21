//! Database Seeders Example
//!
//! Demonstrates database seeding with production safeguards

use rf_testing::{
    factory::{Factory, FactoryDefinition},
    seeder::{DatabaseSeeder, Seeder, SeederError, SeederRunner},
    Fake,
};
use async_trait::async_trait;

// Example models
#[derive(Debug, Clone)]
struct User {
    id: i32,
    name: String,
    email: String,
    role: String,
}

#[derive(Debug, Clone)]
struct Post {
    id: i32,
    user_id: i32,
    title: String,
    body: String,
}

// User Factory
struct UserFactory {
    model: User,
}

impl Default for UserFactory {
    fn default() -> Self {
        Self {
            model: Self::definition(),
        }
    }
}

impl FactoryDefinition for UserFactory {
    type Model = User;

    fn definition() -> Self::Model {
        static ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        User {
            id,
            name: Fake::name(),
            email: format!("user{}@example.com", id),
            role: "user".to_string(),
        }
    }
}

rf_testing::impl_factory!(UserFactory, User);

// User Seeder
struct UserSeeder;

#[async_trait]
impl Seeder for UserSeeder {
    fn name(&self) -> &str {
        "UserSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("   Creating 10 users...");

        for i in 1..=10 {
            let user = UserFactory::new()
                .state(move |u| {
                    u.id = i;
                    u.name = format!("User {}", i);
                    u.email = format!("user{}@example.com", i);
                })
                .create()
                .await
                .map_err(|e| SeederError::SeederFailed(e.to_string()))?;

            println!("     ✓ Created: {} ({})", user.name, user.email);
        }

        Ok(())
    }
}

// Admin Seeder
struct AdminSeeder;

#[async_trait]
impl Seeder for AdminSeeder {
    fn name(&self) -> &str {
        "AdminSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("   Creating admin users...");

        let admin = UserFactory::new()
            .state(|u| {
                u.name = "Admin User".to_string();
                u.email = "admin@example.com".to_string();
                u.role = "admin".to_string();
            })
            .create()
            .await
            .map_err(|e| SeederError::SeederFailed(e.to_string()))?;

        println!("     ✓ Created admin: {} ({})", admin.name, admin.email);

        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["UserSeeder"] // Run after UserSeeder
    }
}

// Post Seeder
struct PostSeeder;

#[async_trait]
impl Seeder for PostSeeder {
    fn name(&self) -> &str {
        "PostSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("   Creating posts...");

        for i in 1..=20 {
            let post = Post {
                id: i,
                user_id: (i % 10) + 1, // Assign to users 1-10
                title: Fake::sentence(5),
                body: Fake::paragraph(3),
            };

            println!("     ✓ Created post {}: {}", post.id, post.title);
        }

        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["UserSeeder"] // Need users first
    }
}

// Conditional Seeder (only runs in development)
struct TestDataSeeder;

#[async_trait]
impl Seeder for TestDataSeeder {
    fn name(&self) -> &str {
        "TestDataSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        println!("   Creating test data...");
        println!("     ✓ Test data created");
        Ok(())
    }

    async fn should_run(&self) -> bool {
        let env = std::env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string());
        env != "production"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌱 Database Seeders Example\n");

    // 1. Basic seeder usage
    println!("1️⃣  Basic DatabaseSeeder usage:");
    println!("   (Running in development mode)\n");

    let seeder = DatabaseSeeder::new()
        .add(UserSeeder)
        .add(PostSeeder)
        .with_environment("development"); // Set environment explicitly

    seeder.run_all().await?;

    // 2. Seeder with dependencies
    println!("\n2️⃣  Seeder with dependencies:");
    println!("   AdminSeeder depends on UserSeeder\n");

    let seeder_with_deps = DatabaseSeeder::new()
        .add(UserSeeder)
        .add(AdminSeeder)
        .with_environment("development");

    seeder_with_deps.run_all().await?;

    // 3. Run specific seeder by name
    println!("\n3️⃣  Run specific seeder by name:");

    let seeder = DatabaseSeeder::new()
        .add(UserSeeder)
        .add(PostSeeder)
        .add(AdminSeeder)
        .with_environment("development");

    seeder.run_by_name("AdminSeeder").await?;

    // 4. Conditional seeder
    println!("\n4️⃣  Conditional seeder (only runs in non-production):");

    let conditional_seeder = DatabaseSeeder::new()
        .add(TestDataSeeder)
        .with_environment("development");

    conditional_seeder.run_all().await?;

    // Switch to production
    println!("\n   Switching to production environment...");
    let prod_seeder = DatabaseSeeder::new()
        .add(TestDataSeeder)
        .with_environment("production");

    prod_seeder.run_all().await?;
    println!("   (TestDataSeeder did not run in production)");

    // 5. SeederRunner with dependency resolution
    println!("\n5️⃣  SeederRunner with automatic dependency resolution:");

    let runner = SeederRunner::new()
        .add_seeder(Box::new(PostSeeder))
        .add_seeder(Box::new(AdminSeeder))
        .add_seeder(Box::new(UserSeeder)); // Add in wrong order

    println!("   Dependencies will be resolved automatically:\n");
    runner.run_all().await?;

    // 6. Production guard demonstration
    println!("\n6️⃣  Production guard demonstration:");
    println!("   When running in production, user must confirm:");
    println!();
    println!("   Example (simulated):");
    println!("   ⚠️  WARNING: You are about to seed the PRODUCTION database!");
    println!("   Environment: production");
    println!("   This will modify production data.");
    println!();
    println!("   Type 'yes' to continue or anything else to cancel: [user input]");
    println!();
    println!("   ✓ Production guard prevents accidental data modification");

    // 7. Disable production guard (dangerous!)
    println!("\n7️⃣  Disabling production guard (use with caution!):");

    let unsafe_seeder = DatabaseSeeder::new()
        .add(UserSeeder)
        .without_production_guard()
        .with_environment("production");

    println!("   ⚠️  Production guard disabled - seeder will run without confirmation");
    println!("   (Not actually running in this example)");

    // 8. Seeder statistics
    println!("\n8️⃣  Seeder statistics:");

    let seeder = DatabaseSeeder::new()
        .add(UserSeeder)
        .add(PostSeeder)
        .add(AdminSeeder);

    println!("   Registered seeders:");
    for name in seeder.seeder_names() {
        println!("     - {}", name);
    }

    // 9. Error handling
    println!("\n9️⃣  Error handling:");

    struct FailingSeeder;

    #[async_trait]
    impl Seeder for FailingSeeder {
        fn name(&self) -> &str {
            "FailingSeeder"
        }

        async fn run(&self) -> Result<(), SeederError> {
            Err(SeederError::SeederFailed("Simulated failure".to_string()))
        }
    }

    let failing_seeder = DatabaseSeeder::new()
        .add(FailingSeeder)
        .with_environment("development");

    match failing_seeder.run_all().await {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => println!("   ✓ Error handled gracefully: {}", e),
    }

    // 10. Best practices
    println!("\n🔟 Best practices:");
    println!("   ✓ Always use production guard in production");
    println!("   ✓ Define dependencies between seeders");
    println!("   ✓ Use conditional seeding for environment-specific data");
    println!("   ✓ Provide clear seeder names");
    println!("   ✓ Handle errors gracefully");
    println!("   ✓ Use factories for test data generation");
    println!("   ✓ Keep seeders idempotent when possible");

    println!("\n✅ All database seeder examples completed successfully!");

    Ok(())
}
