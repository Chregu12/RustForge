# rf-testing - RustForge Testing Framework

Comprehensive testing utilities for RustForge applications, including factories, seeders, fake data generation, and database testing utilities.

## Features

- **Factory System** - Laravel-inspired model factories for generating test data
- **Fake Data Generator** - Comprehensive fake data generation (similar to Faker)
- **Database Seeders** - Structured database seeding with dependency management
- **Testing Utilities** - HTTP testing, assertions, and database test helpers
- **CLI Integration** - Commands for generating factories and seeders

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
rf-testing = { path = "../crates/rf-testing" }
```

## Fake Data Generator

Generate realistic fake data for testing:

```rust
use rf_testing::Fake;

// Names
let name = Fake::name();                    // "John Doe"
let first_name = Fake::first_name();        // "John"
let last_name = Fake::last_name();          // "Doe"
let username = Fake::username();            // "johndoe123"

// Contact
let email = Fake::email();                  // "john@example.com"
let phone = Fake::phone();                  // "(555) 123-4567"

// Address
let address = Fake::address();              // "123 Main St, Springfield, IL 62701"
let city = Fake::city();                    // "Springfield"
let state = Fake::state();                  // "Illinois"
let zip = Fake::zip();                      // "62701"

// Internet
let url = Fake::url();                      // "https://example.com"
let ipv4 = Fake::ipv4();                    // "192.168.1.1"
let user_agent = Fake::user_agent();        // "Mozilla/5.0 ..."

// Text
let word = Fake::word();                    // "lorem"
let sentence = Fake::sentence();            // "Lorem ipsum dolor sit amet."
let paragraph = Fake::paragraph();          // "Lorem ipsum..."
let title = Fake::title();                  // "The Quick Brown Fox"
let slug = Fake::slug();                    // "the-quick-brown-fox"

// Numbers
let number = Fake::number(1, 100);          // Random number between 1 and 100
let float = Fake::float(0.0, 1.0);          // Random float between 0.0 and 1.0
let boolean = Fake::boolean();              // true or false

// Dates
let date = Fake::date();                    // Random date
let datetime = Fake::datetime();            // Random datetime
let past = Fake::past_date(30);             // Date within last 30 days
let future = Fake::future_date(30);         // Date within next 30 days

// Misc
let uuid = Fake::uuid();                    // "550e8400-e29b-41d4-a716-446655440000"
let color = Fake::color_hex();              // "#FF5733"
let company = Fake::company();              // "Acme Corp"
```

## Model Factories

Create test data with factories:

### Basic Factory

```rust
use rf_testing::{Factory, FactoryDefinition, FactoryError, Fake};
use async_trait::async_trait;

#[derive(Clone, Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
    password: String,
    role: String,
    created_at: DateTime<Utc>,
}

struct UserFactory {
    model: User,
}

impl Default for UserFactory {
    fn default() -> Self {
        Self {
            model: <UserFactory as FactoryDefinition>::definition(),
        }
    }
}

impl FactoryDefinition for UserFactory {
    type Model = User;

    fn definition() -> Self::Model {
        User {
            id: 0,
            name: Fake::name(),
            email: Fake::email(),
            password: hash_password("password"),
            role: "user".to_string(),
            created_at: Fake::datetime(),
        }
    }
}

rf_testing::impl_factory!(UserFactory, User);
```

### Using Factories

```rust
// Create a single user
let user = UserFactory::new().create().await?;

// Create with custom state
let admin = UserFactory::new()
    .state(|u| u.role = "admin".to_string())
    .create()
    .await?;

// Build without persisting
let user = UserFactory::new().build();

// Create multiple users
let users = UserFactory::create_many(50).await?;

// Using builder
let users = UserFactory::count(10).create().await?;
```

### Factories with Relationships

```rust
struct PostFactory {
    model: Post,
}

impl FactoryDefinition for PostFactory {
    type Model = Post;

    fn definition() -> Self::Model {
        Post {
            id: 0,
            user_id: 0,  // Will be set via relationship
            title: Fake::title(),
            slug: Fake::slug(),
            body: Fake::paragraphs(3),
            published: true,
            views: Fake::number(0, 1000),
            created_at: Fake::datetime(),
        }
    }
}

impl PostFactory {
    pub fn for_user(mut self, user_id: i32) -> Self {
        self.model.user_id = user_id;
        self
    }
}

// Usage
let user = UserFactory::new().create().await?;
let post = PostFactory::new()
    .for_user(user.id)
    .create()
    .await?;
```

## Database Seeders

Populate your database with test data:

### Basic Seeder

```rust
use rf_testing::{Seeder, SeederError};
use async_trait::async_trait;

pub struct UserSeeder;

#[async_trait]
impl Seeder for UserSeeder {
    fn name(&self) -> &str {
        "UserSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        // Create admin
        UserFactory::new()
            .state(|u| {
                u.email = "admin@example.com".to_string();
                u.role = "admin".to_string();
            })
            .create()
            .await?;

        // Create regular users
        UserFactory::create_many(50).await?;

        Ok(())
    }
}
```

### Seeder with Dependencies

```rust
pub struct PostSeeder;

#[async_trait]
impl Seeder for PostSeeder {
    fn name(&self) -> &str {
        "PostSeeder"
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["UserSeeder"]  // Run UserSeeder first
    }

    async fn run(&self) -> Result<(), SeederError> {
        let users = User::all().await?;

        for user in users {
            let post_count = Fake::number(5, 10) as usize;

            PostFactory::new()
                .for_user(user.id)
                .count(post_count)
                .create()
                .await?;
        }

        Ok(())
    }
}
```

### Running Seeders

```rust
use rf_testing::{SeederRunner};

let runner = SeederRunner::new()
    .add_seeder(Box::new(UserSeeder))
    .add_seeder(Box::new(PostSeeder));

// Run all seeders (respects dependencies)
runner.run_all().await?;

// Run specific seeder
runner.run_one("UserSeeder").await?;
```

## Database Testing Utilities

### Test Database

```rust
use rf_testing::TestDatabase;

#[tokio::test]
async fn test_user_creation() {
    let test_db = TestDatabase::new().await.unwrap();
    test_db.migrate().await.unwrap();

    let user = UserFactory::new().create().await.unwrap();

    assert!(!user.email.is_empty());

    test_db.cleanup().await.unwrap();
}
```

### Test Macro

```rust
use rf_testing::test_with_db;

test_with_db!(test_user_creation, |db| async move {
    let user = UserFactory::new().create().await?;
    assert!(!user.email.is_empty());
    Ok(())
});
```

### Refresh Database

```rust
use rf_testing::refresh_database;

#[tokio::test]
async fn test_with_fresh_db() {
    refresh_database("sqlite::memory:").await.unwrap();

    // Your test code here
}
```

## HTTP Testing

```rust
use rf_testing::HttpTester;
use axum::{Router, routing::get, Json};
use serde_json::json;

#[tokio::test]
async fn test_api_endpoint() {
    let app = Router::new()
        .route("/user", get(get_user));

    let client = HttpTester::new(app);

    client.get("/user")
        .await
        .assert_ok()
        .assert_json(json!({
            "id": 1,
            "name": "Test User"
        }))
        .await;
}
```

## CLI Commands

Generate factories and seeders using the Forge CLI:

### Make Factory

```bash
forge make:factory UserFactory
forge make:factory PostFactory --model=Post
```

This creates a factory file in `tests/factories/user_factory.rs`:

```rust
use rf_testing::{Factory, FactoryDefinition, FactoryError, Fake};
use async_trait::async_trait;
use crate::models::User;

pub struct UserFactory {
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
        User {
            // Fill in your model fields with fake data
            name: Fake::name(),
            email: Fake::email(),
            // ...
        }
    }
}

rf_testing::impl_factory!(UserFactory, User);
```

### Make Seeder

```bash
forge make:seeder UserSeeder
```

This creates a seeder file in `database/seeders/user_seeder.rs`:

```rust
use rf_testing::{Seeder, SeederError};
use async_trait::async_trait;

pub struct UserSeeder;

#[async_trait]
impl Seeder for UserSeeder {
    fn name(&self) -> &str {
        "UserSeeder"
    }

    async fn run(&self) -> Result<(), SeederError> {
        // Implement your seeding logic
        Ok(())
    }
}
```

### Database Seed

```bash
# Run all seeders
forge db:seed

# Run specific seeder
forge db:seed --class=UserSeeder

# Migrate fresh and seed
forge migrate:fresh --seed
```

## Custom Assertions

```rust
use rf_testing::assertions::*;

// Option assertions
assert_some_eq(Some(42), 42);
let value = assert_some(Some(10));
assert_none::<i32>(None);

// Result assertions
assert_ok_eq(Ok::<_, String>(42), 42);
let value = assert_ok(Ok::<_, String>(10));
let err = assert_err(Err::<i32, _>("error"));

// String assertions
assert_contains("Hello, World!", "World");
assert_not_contains("Hello", "Goodbye");

// Range assertions
assert_in_range(5, 1, 10);
```

## Complete Test Example

```rust
use rf_testing::{test_with_db, Factory, Fake};

test_with_db!(test_user_post_relationship, |db| async move {
    // Create user with factory
    let user = UserFactory::new()
        .state(|u| u.email = "test@example.com".to_string())
        .create()
        .await?;

    // Create posts for user
    let posts = PostFactory::new()
        .for_user(user.id)
        .count(5)
        .create()
        .await?;

    // Assertions
    assert_eq!(posts.len(), 5);
    assert_eq!(user.email, "test@example.com");

    for post in posts {
        assert_eq!(post.user_id, user.id);
        assert!(!post.title.is_empty());
    }

    Ok(())
});
```

## API Reference

### Fake Data Generator

All methods are static methods on the `Fake` struct. See the [full list](#fake-data-generator) above.

### Factory Trait

- `definition()` - Define the default state
- `new()` - Create new factory instance
- `state(modifier)` - Modify model state
- `create()` - Create and persist model
- `build()` - Build model without persisting
- `create_many(count)` - Create multiple instances
- `count(n)` - Create factory builder

### Seeder Trait

- `name()` - Get seeder name
- `run()` - Execute seeder
- `should_run()` - Conditional execution
- `dependencies()` - Define dependency order

### SeederRunner

- `new()` - Create runner
- `add_seeder(seeder)` - Add seeder
- `run_all()` - Run all seeders
- `run_one(name)` - Run specific seeder

### TestDatabase

- `new()` - Create test database
- `migrate()` - Run migrations
- `seed()` - Run seeders
- `refresh()` - Refresh database
- `cleanup()` - Clean up

## License

MIT OR Apache-2.0
