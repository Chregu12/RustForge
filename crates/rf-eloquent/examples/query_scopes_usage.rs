//! Query Scopes Usage Examples
//!
//! Demonstrates how to use the query scopes system in RustForge.
//! This shows Laravel-equivalent query scoping functionality.

use rf_eloquent::prelude::*;
use sea_orm::{entity::prelude::*, DatabaseConnection};

// Example User entity with query scopes
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub email_verified_at: Option<DateTimeUtc>,
    pub subscription_tier: String,
    pub created_at: DateTimeUtc,
    pub views: i64,
    pub featured: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Example 1: Define Named Scopes
impl Entity {
    /// Active users scope
    pub fn active<S>(select: S) -> S
    where
        S: QueryFilter,
    {
        select.filter(Column::Active.eq(true))
    }

    /// Verified users scope
    pub fn verified<S>(select: S) -> S
    where
        S: QueryFilter,
    {
        select.filter(Column::EmailVerifiedAt.is_not_null())
    }

    /// Premium users scope
    pub fn premium<S>(select: S) -> S
    where
        S: QueryFilter,
    {
        select.filter(Column::SubscriptionTier.eq("premium"))
    }

    /// Popular users scope (parameterized)
    pub fn popular<S>(select: S, min_views: i64) -> S
    where
        S: QueryFilter,
    {
        select.filter(Column::Views.gt(min_views))
    }

    /// Recent users scope (created in last N days)
    pub fn recent<S>(select: S, days: i64) -> S
    where
        S: QueryFilter,
    {
        use chrono::{Duration, Utc};
        let threshold = Utc::now() - Duration::days(days);
        select.filter(Column::CreatedAt.gt(threshold))
    }

    /// Featured users scope
    pub fn featured<S>(select: S) -> S
    where
        S: QueryFilter,
    {
        select.filter(Column::Featured.eq(true))
    }
}

/// Example 2: Using Scopes
async fn example_using_scopes(db: &DatabaseConnection) -> Result<(), DbErr> {
    // Single scope
    let active_users = Entity::find().apply_if(Entity::active).all(db).await?;

    println!("Found {} active users", active_users.len());

    // Chaining multiple scopes
    let verified_premium_users = Entity::find()
        .apply_if(Entity::active)
        .apply_if(Entity::verified)
        .apply_if(Entity::premium)
        .all(db)
        .await?;

    println!(
        "Found {} verified premium users",
        verified_premium_users.len()
    );

    // Parameterized scope
    let popular_users = Entity::find()
        .apply_if(|q| Entity::popular(q, 1000))
        .all(db)
        .await?;

    println!("Found {} popular users", popular_users.len());

    Ok(())
}

/// Example 3: Conditional Scopes
async fn example_conditional_scopes(
    db: &DatabaseConnection,
    filter_by_premium: bool,
    min_views: Option<i64>,
) -> Result<(), DbErr> {
    let mut query = Entity::find().apply_if(Entity::active);

    // Apply premium filter conditionally
    query = query.apply_when(filter_by_premium, Entity::premium);

    // Apply views filter if threshold provided
    if let Some(threshold) = min_views {
        query = query.apply_if(|q| Entity::popular(q, threshold));
    }

    let users = query.all(db).await?;
    println!("Found {} users with filters", users.len());

    Ok(())
}

/// Example 4: Using ScopeBuilder
async fn example_scope_builder(db: &DatabaseConnection) -> Result<(), DbErr> {
    let builder = ScopeBuilder::<Entity>::new()
        .scope("active", Entity::active)
        .scope("verified", Entity::verified)
        .when(true, "featured", Entity::featured);

    println!("Applied scopes: {:?}", builder.get_applied_scopes());

    let users = builder.get(db).await?;
    println!("Found {} users", users.len());

    Ok(())
}

/// Example 5: Using CommonScopes
async fn example_common_scopes(db: &DatabaseConnection) -> Result<(), DbErr> {
    // Active records
    let active = CommonScopes::active::<Entity, _, _>(Entity::find(), Column::Active)
        .all(db)
        .await?;

    // Recent records (last 7 days)
    let recent = CommonScopes::recent::<Entity, _, _>(Entity::find(), Column::CreatedAt, 7)
        .all(db)
        .await?;

    // Popular records
    let popular = CommonScopes::popular::<Entity, _, _>(Entity::find(), Column::Views, 1000)
        .all(db)
        .await?;

    // Verified records
    let verified = CommonScopes::verified::<Entity, _, _>(Entity::find(), Column::EmailVerifiedAt)
        .all(db)
        .await?;

    // Featured records
    let featured = CommonScopes::featured::<Entity, _, _>(Entity::find(), Column::Featured)
        .all(db)
        .await?;

    // Latest records (ordered by created_at DESC)
    let latest = CommonScopes::latest::<Entity, _, _>(Entity::find(), Column::CreatedAt)
        .all(db)
        .await?;

    println!(
        "Active: {}, Recent: {}, Popular: {}, Verified: {}, Featured: {}, Latest: {}",
        active.len(),
        recent.len(),
        popular.len(),
        verified.len(),
        featured.len(),
        latest.len()
    );

    Ok(())
}

/// Example 6: Global Scopes
fn example_global_scopes() {
    let mut registry = GlobalScopeRegistry::<Entity>::new();

    // Register global scope that applies to all queries
    registry.register("active_by_default", |select| Entity::active(select));

    // All queries will now have the active scope applied
    let query = Entity::find();
    let _scoped_query = registry.apply_all(query);

    // Remove global scope when needed
    registry.remove("active_by_default");

    println!("Global scopes registered: {}", registry.count());
}

/// Example 7: Complex Scope Combinations
async fn example_complex_scopes(db: &DatabaseConnection) -> Result<(), DbErr> {
    // Combine scopes with other query builder methods
    let users = Entity::find()
        .apply_if(Entity::active)
        .apply_if(Entity::verified)
        .filter(Column::SubscriptionTier.is_in(vec!["premium", "enterprise"]))
        .order_by_desc(Column::CreatedAt)
        .limit(10)
        .all(db)
        .await?;

    println!("Found {} users with complex filters", users.len());

    Ok(())
}

/// Example 8: Reusable Scope Functions
fn example_reusable_scopes() {
    // Scopes can be stored and reused
    let active_verified =
        |query: sea_orm::Select<Entity>| query.apply_if(Entity::active).apply_if(Entity::verified);

    // Use the combined scope
    let _query = Entity::find().apply_if(active_verified);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Query Scopes Usage Examples");
    println!("============================\n");

    // Note: These examples require a database connection
    // For demonstration, we show the API without actually connecting

    println!("1. Named Scopes - Define reusable query filters");
    println!("2. Using Scopes - Apply scopes to queries");
    println!("3. Conditional Scopes - Apply scopes based on conditions");
    println!("4. ScopeBuilder - Build queries with tracked scopes");
    println!("5. CommonScopes - Use pre-built common scopes");
    println!("6. Global Scopes - Automatically apply scopes to all queries");
    println!("7. Complex Combinations - Mix scopes with other query methods");
    println!("8. Reusable Functions - Create and reuse scope combinations");

    example_global_scopes();
    example_reusable_scopes();

    println!("\nAll examples demonstrated successfully!");

    Ok(())
}
