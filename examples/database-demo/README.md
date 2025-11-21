# Database Demo - Complete CRUD Example

Comprehensive demonstration of **rf-orm** ORM integration with SeaORM.

## Features Demonstrated

✅ **Database Connection Management**
- Connection pooling with `DatabaseManager`
- SQLite in-memory database (no setup required)
- Configuration with `DatabaseConfig`

✅ **Entity Definition**
- SeaORM entity with `DeriveEntityModel`
- Type-safe column definitions
- Serialization with Serde
- Timestamp fields (created_at, updated_at)
- Soft delete support (deleted_at)

✅ **CRUD Operations**
- **Create**: Insert new records
- **Read**: Find by ID, find all, query with filters
- **Update**: Update existing records
- **Delete**: Soft delete and hard delete

✅ **Query Features**
- Filter by column values
- Order by columns (ascending/descending)
- Count records
- Exclude soft-deleted records

✅ **Soft Delete**
- Mark records as deleted without removing them
- Query active vs. deleted records
- Restore soft-deleted records
- Implement `SoftDelete` trait

## Quick Start

```bash
# Run the demo
cargo run -p database-demo
```

## Demo Steps

The example walks through 16 steps:

### 1. **Connect to Database**
```rust
let config = DatabaseConfig {
    url: "sqlite::memory:".to_string(),
    max_connections: 5,
    min_connections: 1,
    ..Default::default()
};

let db = DatabaseManager::connect(config).await?;
```

### 2. **Create Table**
```rust
let schema = Schema::new(DbBackend::Sqlite);
let stmt = schema.create_table_from_entity(User);
db.execute(stmt).await?;
```

### 3. **Insert Users**
```rust
let user = user::ActiveModel {
    email: Set("alice@example.com".to_string()),
    name: Set("Alice Smith".to_string()),
    password_hash: Set("$2b$12$...".to_string()),
    created_at: Set(Utc::now()),
    updated_at: Set(Utc::now()),
    deleted_at: Set(None),
    ..Default::default()
};

let result = User::insert(user).exec(db).await?;
```

### 4. **Query All Users**
```rust
let users = User::find().all(db).await?;
```

### 5. **Find by ID**
```rust
let user = User::find_by_id(1)
    .one(db)
    .await?
    .expect("User not found");
```

### 6. **Update User**
```rust
let mut user_active: user::ActiveModel = user.into();
user_active.name = Set("New Name".to_string());
user_active.updated_at = Set(Utc::now());
let updated = user_active.update(db).await?;
```

### 7. **Query with Filter**
```rust
let filtered = User::find()
    .filter(user::Column::Email.contains("alice"))
    .all(db)
    .await?;
```

### 8. **Soft Delete**
```rust
let mut user_active: user::ActiveModel = user.into();
user_active.soft_delete();  // Sets deleted_at timestamp
let soft_deleted = user_active.update(db).await?;
```

### 9. **Query Active Users**
```rust
let active = User::find()
    .filter(user::Column::DeletedAt.is_null())
    .all(db)
    .await?;
```

### 10. **Query Soft-Deleted Users**
```rust
let deleted = User::find()
    .filter(user::Column::DeletedAt.is_not_null())
    .all(db)
    .await?;
```

### 11. **Restore Soft-Deleted**
```rust
let mut user_active: user::ActiveModel = user.into();
user_active.restore();  // Clears deleted_at
let restored = user_active.update(db).await?;
```

### 12. **Order By**
```rust
let ordered = User::find()
    .order_by_desc(user::Column::CreatedAt)
    .all(db)
    .await?;
```

### 13. **Count Records**
```rust
let count = User::find().count(db).await?;
```

### 14. **Hard Delete**
```rust
let result = User::delete_by_id(user_id)
    .exec(db)
    .await?;
```

## Entity Definition

```rust
use rf_orm::{SoftDelete, Set};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub email: String,

    pub name: String,

    #[serde(skip_serializing)]
    pub password_hash: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    #[sea_orm(nullable)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl SoftDelete for ActiveModel {
    fn soft_delete(&mut self) {
        self.deleted_at = Set(Some(Utc::now()));
        self.updated_at = Set(Utc::now());
    }

    fn restore(&mut self) {
        self.deleted_at = Set(None);
        self.updated_at = Set(Utc::now());
    }

    fn is_deleted(&self) -> bool {
        matches!(
            &self.deleted_at,
            ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_))
        )
    }
}
```

## Output

When you run the demo, you'll see output like:

```
🚀 Database Demo - rf-orm with SeaORM
================================================

📦 Step 1: Connecting to database...
✅ Connected successfully

🔨 Step 2: Creating users table...
✅ Table created

➕ Step 3: Inserting users...
   Created user: Alice Smith <alice@example.com> (id: 1)
   Created user: Bob Jones <bob@example.com> (id: 2)
   Created user: Charlie Brown <charlie@example.com> (id: 3)

🔍 Step 4: Querying all users...
   Found 3 users:
   - Alice Smith <alice@example.com>
   - Bob Jones <bob@example.com>
   - Charlie Brown <charlie@example.com>

🔎 Step 5: Finding user by ID...
   Found: Alice Smith <alice@example.com>

✏️  Step 6: Updating user name...
   Updated to: Alice Johnson <alice@example.com>

🔍 Step 7: Querying users with email filter...
   Found 1 user(s) matching filter:
   - Alice Johnson <alice@example.com>

🗑️  Step 8: Soft deleting user (Bob)...
   Soft deleted: Bob Jones <bob@example.com> (deleted_at: Some(...))

🔍 Step 9: Querying active users (excluding soft-deleted)...
   Found 2 active user(s):
   - Alice Johnson <alice@example.com>
   - Charlie Brown <charlie@example.com>

🔍 Step 10: Querying soft-deleted users...
   Found 1 soft-deleted user(s):
   - Bob Jones <bob@example.com> (deleted_at: Some(...))

♻️  Step 11: Restoring soft-deleted user...
   Restored: Bob Jones <bob@example.com> (deleted_at: None)

🔍 Step 12: Querying users ordered by creation date...
   Users ordered by created_at (newest first):
   - Charlie Brown <charlie@example.com>
   - Bob Jones <bob@example.com>
   - Alice Johnson <alice@example.com>

🔢 Step 13: Counting total users...
   Total users: 3

🗑️  Step 14: Hard deleting user (Charlie)...
   Deleted 1 row(s)

🔢 Step 15: Final user count...
   Remaining users: 2

📋 Step 16: Final user list...
   - Alice Johnson <alice@example.com>
   - Bob Jones <bob@example.com>

✅ Demo completed successfully!
================================================
```

## Key Concepts

### Active Model Pattern

SeaORM uses the Active Model pattern where:
- `Model` = Data from database (read-only)
- `ActiveModel` = Editable version with change tracking
- `Entity` = Database table interface

```rust
// Convert Model to ActiveModel for editing
let mut active: user::ActiveModel = model.into();

// Set new values
active.name = Set("New Name".to_string());

// Save changes
let updated = active.update(db).await?;
```

### Soft Delete Benefits

1. **Data Recovery**: Restore accidentally deleted records
2. **Audit Trail**: Keep history of deletions
3. **Referential Integrity**: Maintain relationships
4. **Compliance**: Meet data retention requirements

### Query Builder

SeaORM provides a type-safe query builder:

```rust
User::find()
    .filter(user::Column::Email.contains("@example.com"))
    .filter(user::Column::DeletedAt.is_null())
    .order_by_desc(user::Column::CreatedAt)
    .limit(10)
    .all(db)
    .await?;
```

## Using with Real Databases

### SQLite File

```rust
let config = DatabaseConfig {
    url: "sqlite://./database.db".to_string(),
    ..Default::default()
};
```

### PostgreSQL

```rust
let config = DatabaseConfig {
    url: "postgres://user:pass@localhost/mydb".to_string(),
    max_connections: 20,
    ..Default::default()
};
```

### MySQL

```rust
let config = DatabaseConfig {
    url: "mysql://user:pass@localhost/mydb".to_string(),
    ..Default::default()
};
```

## Next Steps

- Add migrations with `sea-orm-migration`
- Implement relationships (one-to-many, many-to-many)
- Add pagination for large datasets
- Integrate with Axum web framework
- Add transaction support for complex operations

## See Also

- [SeaORM Documentation](https://www.sea-ql.org/SeaORM/)
- [rf-orm Documentation](../../crates/rf-orm/)
- [API Sketch](../../docs/api-skizzen/03-rf-orm-database-integration.md)
