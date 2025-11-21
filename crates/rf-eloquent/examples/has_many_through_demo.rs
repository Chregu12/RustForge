//! Demonstration of has_many_through relationship
//!
//! This example shows how to use the has_many_through query helper
//! to load related models through an intermediate table.
//!
//! Example: Country -> Users -> Posts
//! Get all posts for a country through its users

use rf_eloquent::has_many_through;
use sea_orm::{
    entity::prelude::*, Database, DatabaseBackend, DbErr,
    Schema, Set,
};

// ============================================================================
// Entity Definitions
// ============================================================================

// Country entity
mod country {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "countries")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// User entity (through table)
mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub country_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// Post entity (final table)
mod post {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub content: String,
        pub user_id: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Main Demo
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    // Setup in-memory database
    let db = Database::connect("sqlite::memory:").await?;

    // Create schema
    let schema = Schema::new(DatabaseBackend::Sqlite);

    db.execute(db.get_database_backend().build(&schema.create_table_from_entity(country::Entity))).await?;
    db.execute(db.get_database_backend().build(&schema.create_table_from_entity(user::Entity))).await?;
    db.execute(db.get_database_backend().build(&schema.create_table_from_entity(post::Entity))).await?;

    println!("Database schema created successfully!\n");

    // Insert test data
    println!("Inserting test data...");

    // Create countries
    let usa = country::ActiveModel {
        id: Set(1),
        name: Set("USA".to_string()),
    };
    usa.insert(&db).await?;

    let canada = country::ActiveModel {
        id: Set(2),
        name: Set("Canada".to_string()),
    };
    canada.insert(&db).await?;

    // Create users in USA
    let alice = user::ActiveModel {
        id: Set(1),
        name: Set("Alice".to_string()),
        country_id: Set(1),
    };
    alice.insert(&db).await?;

    let bob = user::ActiveModel {
        id: Set(2),
        name: Set("Bob".to_string()),
        country_id: Set(1),
    };
    bob.insert(&db).await?;

    // Create users in Canada
    let charlie = user::ActiveModel {
        id: Set(3),
        name: Set("Charlie".to_string()),
        country_id: Set(2),
    };
    charlie.insert(&db).await?;

    // Create posts for Alice (USA)
    for i in 1..=3 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Alice's Post {}", i)),
            content: Set("Content from USA".to_string()),
            user_id: Set(1),
        };
        post.insert(&db).await?;
    }

    // Create posts for Bob (USA)
    for i in 4..=5 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Bob's Post {}", i - 3)),
            content: Set("More content from USA".to_string()),
            user_id: Set(2),
        };
        post.insert(&db).await?;
    }

    // Create posts for Charlie (Canada)
    for i in 6..=8 {
        let post = post::ActiveModel {
            id: Set(i),
            title: Set(format!("Charlie's Post {}", i - 5)),
            content: Set("Content from Canada".to_string()),
            user_id: Set(3),
        };
        post.insert(&db).await?;
    }

    println!("Test data inserted successfully!\n");

    // ============================================================================
    // Demonstrate has_many_through
    // ============================================================================

    println!("=== Has-Many-Through Demonstration ===\n");

    // Get all posts for USA (through users)
    println!("Loading all posts for USA (through users)...");
    let usa_posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        1, // USA country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await?;

    println!("Found {} posts from USA:", usa_posts.len());
    for post in &usa_posts {
        println!("  - ID {}: {}", post.id, post.title);
    }
    println!();

    // Get all posts for Canada (through users)
    println!("Loading all posts for Canada (through users)...");
    let canada_posts = has_many_through::<post::Entity, user::Entity, post::Model, _>(
        &db,
        2, // Canada country_id
        user::Column::CountryId,
        post::Column::UserId,
        user::Column::Id,
    )
    .await?;

    println!("Found {} posts from Canada:", canada_posts.len());
    for post in &canada_posts {
        println!("  - ID {}: {}", post.id, post.title);
    }
    println!();

    // Verify results
    println!("=== Verification ===");
    println!("Total posts in database: {}", usa_posts.len() + canada_posts.len());
    println!("USA posts: {}", usa_posts.len());
    println!("Canada posts: {}", canada_posts.len());
    println!();

    // Show SQL concept
    println!("=== SQL Explanation ===");
    println!("The has_many_through query executes SQL equivalent to:");
    println!("SELECT posts.* FROM posts");
    println!("WHERE posts.user_id IN (");
    println!("    SELECT users.id FROM users");
    println!("    WHERE users.country_id = ?");
    println!(")");
    println!();

    println!("SUCCESS! has_many_through executes REAL database queries!");

    Ok(())
}
