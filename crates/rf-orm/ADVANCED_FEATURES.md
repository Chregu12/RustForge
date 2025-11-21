# Advanced Database Features - RustForge ORM

This document covers the advanced database features implemented in RustForge ORM, bringing enterprise-grade capabilities to your applications.

## Table of Contents

1. [Advanced Migrations](#advanced-migrations)
2. [Database Sharding](#database-sharding)
3. [Performance Considerations](#performance-considerations)
4. [Database Compatibility](#database-compatibility)

---

## Advanced Migrations

Advanced migration features provide sophisticated schema management capabilities including foreign keys, indexes, unique constraints, and more.

### Features

- ✅ Foreign key constraints with cascade actions
- ✅ Single and composite indexes
- ✅ Unique constraints
- ✅ Check constraints
- ✅ Table and column renaming
- ✅ Database backend-aware operations

### Basic Usage

```rust
use rf_orm::prelude::*;
use rf_orm::advanced_migrations::*;

async fn run_advanced_migration(db: &DatabaseConnection) -> Result<(), AdvancedMigrationError> {
    let builder = AdvancedMigrationBuilder::new(db);

    // Create an index
    builder.create_index(
        "users",
        vec!["email"],
        true  // unique index
    ).await?;

    // Create composite index
    builder.create_index(
        "posts",
        vec!["user_id", "created_at"],
        false  // non-unique
    ).await?;

    // Add unique constraint
    builder.add_unique_constraint(
        "user_roles",
        vec!["user_id", "role_id"]
    ).await?;

    Ok(())
}
```

### Foreign Keys (PostgreSQL/MySQL)

**Important**: SQLite does not support adding foreign keys to existing tables. Foreign keys must be defined in the CREATE TABLE statement for SQLite.

```rust
// PostgreSQL/MySQL only
async fn add_foreign_keys(db: &DatabaseConnection) -> AdvancedMigrationResult<()> {
    let builder = AdvancedMigrationBuilder::new(db);

    // Basic foreign key
    builder.add_foreign_key(
        "posts",
        vec!["user_id"],
        "users",
        vec!["id"],
        None,
        None
    ).await?;

    // Foreign key with CASCADE on delete
    builder.add_foreign_key(
        "comments",
        vec!["post_id"],
        "posts",
        vec!["id"],
        Some(ForeignKeyAction::Cascade),
        None
    ).await?;

    // Foreign key with SET NULL on delete
    builder.add_foreign_key(
        "orders",
        vec!["customer_id"],
        "customers",
        vec!["id"],
        Some(ForeignKeyAction::SetNull),
        None
    ).await?;

    Ok(())
}
```

### Available Foreign Key Actions

```rust
pub enum ForeignKeyAction {
    Cascade,     // Delete/update cascades to child rows
    SetNull,     // Set foreign key to NULL
    Restrict,    // Reject the operation
    NoAction,    // Same as Restrict but deferred
    SetDefault,  // Set to default value
}
```

### Indexes

```rust
async fn create_indexes(db: &DatabaseConnection) -> AdvancedMigrationResult<()> {
    let builder = AdvancedMigrationBuilder::new(db);

    // Single column index
    builder.create_index("users", vec!["email"], false).await?;

    // Composite index
    builder.create_index(
        "orders",
        vec!["customer_id", "order_date"],
        false
    ).await?;

    // Unique index
    builder.create_index("users", vec!["username"], true).await?;

    // Named index
    builder.create_named_index(
        "products",
        "idx_product_search",
        vec!["name", "category"],
        false
    ).await?;

    // Drop index
    builder.drop_index("users", "idx_users_email").await?;

    Ok(())
}
```

### Check Constraints

```rust
async fn add_check_constraints(db: &DatabaseConnection) -> AdvancedMigrationResult<()> {
    let builder = AdvancedMigrationBuilder::new(db);

    // Age validation
    builder.add_check_constraint(
        "users",
        "chk_age_positive",
        "age >= 0"
    ).await?;

    // Price validation
    builder.add_check_constraint(
        "products",
        "chk_price_positive",
        "price > 0"
    ).await?;

    // Drop check constraint
    builder.drop_check_constraint("users", "chk_age_positive").await?;

    Ok(())
}
```

### Table Operations

```rust
async fn table_operations(db: &DatabaseConnection) -> AdvancedMigrationResult<()> {
    let builder = AdvancedMigrationBuilder::new(db);

    // Rename table
    builder.rename_table("old_users", "new_users").await?;

    // Rename column (MySQL requires column type)
    builder.rename_column(
        "users",
        "name",
        "full_name",
        Some("VARCHAR(255)")  // Required for MySQL
    ).await?;

    // Drop column
    builder.drop_column("users", "temp_field").await?;

    Ok(())
}
```

### Complete Migration Example

```rust
use rf_orm::migrations::*;
use rf_orm::advanced_migrations::*;
use async_trait::async_trait;

pub struct CreatePostsWithConstraints;

#[async_trait]
impl Migration for CreatePostsWithConstraints {
    fn name(&self) -> &str {
        "2025_01_01_000001_create_posts_with_constraints"
    }

    async fn up(&self, schema: &SchemaContext) -> MigrationResult<()> {
        // Create tables
        schema.create("users", |table| {
            table.id();
            table.string("email").unique();
            table.string("username");
            table.timestamps();
        }).await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;

        schema.create("posts", |table| {
            table.id();
            table.string("title");
            table.text("body");
            table.big_integer("user_id").unsigned();
            table.integer("views").default("0");
            table.timestamps();
        }).await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;

        // Get database connection from schema context
        let db = schema.connection();
        let builder = AdvancedMigrationBuilder::new(db);

        // Add indexes
        builder.create_index("users", vec!["username"], true).await
            .map_err(|e| MigrationError::SchemaError(e.to_string()))?;

        builder.create_index("posts", vec!["user_id", "created_at"], false).await
            .map_err(|e| MigrationError::SchemaError(e.to_string()))?;

        // For PostgreSQL/MySQL, add foreign key
        if db.get_database_backend() != DbBackend::Sqlite {
            builder.add_foreign_key(
                "posts",
                vec!["user_id"],
                "users",
                vec!["id"],
                Some(ForeignKeyAction::Cascade),
                None
            ).await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
        }

        Ok(())
    }

    async fn down(&self, schema: &SchemaContext) -> MigrationResult<()> {
        schema.drop("posts").await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
        schema.drop("users").await.map_err(|e| MigrationError::SchemaError(e.to_string()))?;
        Ok(())
    }
}
```

---

## Database Sharding

Database sharding enables horizontal scaling by distributing data across multiple database instances.

### When to Use Sharding

- ✅ Multi-tenant applications (tenant per shard)
- ✅ Large datasets requiring horizontal partitioning
- ✅ Geographic data distribution
- ✅ High-traffic applications needing scale-out
- ✅ Compliance requirements (data residency)

### Available Strategies

1. **Hash Strategy** - Distribute data evenly using consistent hashing
2. **Range Strategy** - Route data based on ID ranges
3. **Tenant Strategy** - Explicit tenant-to-shard mapping
4. **Geographic Strategy** - Region-based routing

### Hash-Based Sharding

Perfect for general-purpose sharding with uniform distribution.

```rust
use rf_orm::sharding::*;
use std::sync::Arc;

async fn setup_hash_sharding() -> ShardResult<ShardManager> {
    // Create strategy
    let strategy = HashStrategy::new(vec![
        "shard_1".to_string(),
        "shard_2".to_string(),
        "shard_3".to_string(),
    ]);

    // Create manager
    let mut manager = ShardManager::new(Arc::new(strategy));

    // Connect to databases
    let db1 = Database::connect("postgres://localhost/shard_1").await?;
    let db2 = Database::connect("postgres://localhost/shard_2").await?;
    let db3 = Database::connect("postgres://localhost/shard_3").await?;

    // Register shards
    manager.add_shard("shard_1".to_string(), Arc::new(db1));
    manager.add_shard("shard_2".to_string(), Arc::new(db2));
    manager.add_shard("shard_3".to_string(), Arc::new(db3));

    Ok(manager)
}

// Use sharding
async fn find_user(manager: &ShardManager, user_id: i64) -> ShardResult<Option<User>> {
    let db = manager.connection_for(&user_id.to_string()).await?;

    let user = User::find_by_id(user_id)
        .one(db.as_ref())
        .await?;

    Ok(user)
}
```

### Range-Based Sharding

Ideal for time-series data or when you want explicit control over distribution.

```rust
async fn setup_range_sharding() -> ShardResult<ShardManager> {
    // Define ranges: users 1-1M on shard1, 1M-2M on shard2, etc.
    let strategy = RangeStrategy::new(vec![
        (1, 1_000_000, "shard_1".to_string()),
        (1_000_001, 2_000_000, "shard_2".to_string()),
        (2_000_001, 3_000_000, "shard_3".to_string()),
    ]);

    let mut manager = ShardManager::new(Arc::new(strategy));

    // Add shards...

    Ok(manager)
}

// Query specific range
async fn get_users_in_range(manager: &ShardManager) -> ShardResult<Vec<User>> {
    // User ID 1,500,000 will automatically route to shard_2
    let db = manager.connection_for("1500000").await?;

    let users = User::find()
        .filter(user::Column::Id.between(1_000_001, 1_500_000))
        .all(db.as_ref())
        .await?;

    Ok(users)
}
```

### Tenant-Based Sharding

Perfect for multi-tenant SaaS applications with explicit tenant placement.

```rust
use std::collections::HashMap;

async fn setup_tenant_sharding() -> ShardResult<ShardManager> {
    let mut tenant_map = HashMap::new();

    // Premium customers get dedicated shards
    tenant_map.insert("tenant_acme".to_string(), "shard_premium_1".to_string());
    tenant_map.insert("tenant_bigcorp".to_string(), "shard_premium_2".to_string());

    // Standard customers share a shard
    tenant_map.insert("tenant_startup_a".to_string(), "shard_standard".to_string());
    tenant_map.insert("tenant_startup_b".to_string(), "shard_standard".to_string());

    // Create strategy with default shard for new tenants
    let strategy = TenantStrategy::with_default(
        tenant_map,
        "shard_standard".to_string()
    );

    let mut manager = ShardManager::new(Arc::new(strategy));

    // Add shards...

    Ok(manager)
}

// Query tenant data
async fn get_tenant_users(
    manager: &ShardManager,
    tenant_id: &str
) -> ShardResult<Vec<User>> {
    let db = manager.connection_for(tenant_id).await?;

    let users = User::find()
        .filter(user::Column::TenantId.eq(tenant_id))
        .all(db.as_ref())
        .await?;

    Ok(users)
}
```

### Geographic Sharding

Route users to regional databases for compliance and performance.

```rust
async fn setup_geographic_sharding() -> ShardResult<ShardManager> {
    let mut region_map = HashMap::new();
    region_map.insert("US".to_string(), "shard_us_east".to_string());
    region_map.insert("EU".to_string(), "shard_eu_west".to_string());
    region_map.insert("APAC".to_string(), "shard_asia_pacific".to_string());

    let strategy = GeographicStrategy::with_default(
        region_map,
        "shard_global".to_string()
    );

    let mut manager = ShardManager::new(Arc::new(strategy));

    // Connect to regional databases...

    Ok(manager)
}

// Query by region
async fn get_regional_data(
    manager: &ShardManager,
    region: &str
) -> ShardResult<Vec<User>> {
    let db = manager.connection_for(region).await?;

    let users = User::find()
        .filter(user::Column::Region.eq(region))
        .all(db.as_ref())
        .await?;

    Ok(users)
}
```

### Cross-Shard Operations

Execute queries across all shards for global operations.

```rust
async fn count_total_users(manager: &ShardManager) -> ShardResult<i64> {
    let counts: Vec<i64> = manager.execute_on_all(|db| {
        Box::pin(async move {
            let count = User::find().count(db).await?;
            Ok(count as i64)
        })
    }).await?;

    let total: i64 = counts.iter().sum();
    Ok(total)
}

// Execute on specific shards only
async fn backup_premium_shards(manager: &ShardManager) -> ShardResult<()> {
    let premium_shards = vec!["shard_premium_1".to_string(), "shard_premium_2".to_string()];

    manager.execute_on_shards(premium_shards, |db| {
        Box::pin(async move {
            // Perform backup operation
            Ok(())
        })
    }).await?;

    Ok(())
}
```

### Convenience Methods

```rust
// Execute with automatic shard selection
async fn create_user(manager: &ShardManager, user_id: i64, name: String) -> ShardResult<User> {
    manager.execute_with_key(&user_id.to_string(), |db| {
        Box::pin(async move {
            let user = user::ActiveModel {
                id: Set(user_id),
                name: Set(name),
                ..Default::default()
            };

            user.insert(db).await
        })
    }).await
}

// Get shard information
fn inspect_sharding(manager: &ShardManager) {
    println!("Total shards: {}", manager.shard_count());
    println!("Shard names: {:?}", manager.shard_names());

    if manager.has_shard("shard_1") {
        println!("Shard 1 is registered");
    }
}
```

---

## Performance Considerations

### Advanced Migrations

- **Indexes**: Create indexes on frequently queried columns
- **Composite Indexes**: Use for queries that filter on multiple columns
- **Foreign Keys**: Add overhead but ensure data integrity
- **Batch Operations**: Group multiple migration operations when possible

### Database Sharding

- **Shard Key Selection**: Choose a shard key that distributes data evenly
- **Cross-Shard Queries**: Minimize these as they're expensive
- **Connection Pooling**: Each shard maintains its own connection pool
- **Consistent Hashing**: Hash strategy uses consistent hashing for even distribution

### Performance Tips

```rust
// Good: Single shard query
let user = manager.connection_for(&user_id.to_string()).await?;
let posts = Post::find().filter(post::Column::UserId.eq(user_id)).all(user.as_ref()).await?;

// Avoid: Cross-shard aggregations unless necessary
let total_posts = manager.execute_on_all(|db| {
    Box::pin(async move { Post::find().count(db).await })
}).await?;  // Queries all shards!

// Better: Cache aggregated data
```

---

## Database Compatibility

### Advanced Migrations

| Feature | PostgreSQL | MySQL | SQLite |
|---------|-----------|--------|---------|
| Foreign Keys (ALTER) | ✅ | ✅ | ❌ * |
| Indexes | ✅ | ✅ | ✅ |
| Unique Constraints | ✅ | ✅ | ✅ ** |
| Check Constraints | ✅ | ✅ | ⚠️ *** |
| Rename Table | ✅ | ✅ | ✅ |
| Rename Column | ✅ | ✅ | ✅ |
| Drop Column | ✅ | ✅ | ⚠️ **** |

\* SQLite requires foreign keys to be defined in CREATE TABLE
\** SQLite uses unique indexes instead of constraints
\*** SQLite has limited check constraint support
\**** SQLite version 3.35.0+ required

### Database Sharding

All sharding strategies work with any database backend supported by SeaORM:

- PostgreSQL
- MySQL/MariaDB
- SQLite
- SQL Server (via SeaORM)

---

## Testing

Both features include comprehensive test suites:

- **Advanced Migrations**: 20 tests covering all features
- **Database Sharding**: 24 tests covering all strategies

Run tests:

```bash
# Advanced migrations tests
cargo test --test advanced_migrations_tests

# Sharding tests
cargo test --test sharding_tests

# All tests
cargo test
```

---

## Best Practices

### Migrations

1. **Always Test Migrations**: Test both up() and down() on development data
2. **Use Transactions**: Wrap complex migrations in transactions
3. **Index Strategy**: Add indexes after bulk data loads for better performance
4. **Foreign Keys**: Use appropriate cascade actions to maintain referential integrity
5. **Database-Specific Code**: Check database backend when using features not supported everywhere

### Sharding

1. **Choose the Right Strategy**: Select based on your data access patterns
2. **Shard Key Immutability**: Never change shard keys after assignment
3. **Monitor Distribution**: Regularly check that data is evenly distributed
4. **Plan for Growth**: Design sharding strategy to accommodate future growth
5. **Test Failover**: Implement and test shard failover strategies
6. **Document Topology**: Maintain clear documentation of shard topology

---

## Examples

See the `examples/` directory for complete working examples:

- `examples/advanced_migrations.rs` - Migration examples
- `examples/hash_sharding.rs` - Hash-based sharding
- `examples/tenant_sharding.rs` - Multi-tenant sharding
- `examples/geographic_sharding.rs` - Region-based sharding

---

## License

MIT License - See LICENSE file for details
