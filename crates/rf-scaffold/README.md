# rf-scaffold

Code scaffolding and generation system for RustForge, inspired by Laravel's artisan make commands.

## Features

- **Project Scaffolding**: Create complete project structures with different templates (API, full-stack, microservice, CLI)
- **Code Generation**: Generate models, controllers, migrations, services, and repositories
- **Template Engine**: Handlebars-based templates with variable substitution
- **Custom Templates**: Register your own templates for project-specific needs
- **Smart Naming**: Automatic pluralization, snake_case, PascalCase, kebab-case conversions
- **Zero Configuration**: Works out of the box with sensible defaults

## Quick Start

```rust
use rf_scaffold::{ScaffoldEngine, ModelOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scaffold = ScaffoldEngine::new("./my-project")?;

    // Generate a model
    scaffold.generate_model("User", &ModelOptions {
        fields: vec![
            ("name", "String"),
            ("email", "String"),
            ("age", "i32"),
        ],
        with_migration: true,
        with_factory: false,
    }).await?;

    // Generate a controller
    scaffold.generate_controller("UserController", false).await?;

    // Generate a migration
    scaffold.generate_migration("create_users_table").await?;

    Ok(())
}
```

## Code Generators

### Model Generator

Generate a complete model with fields, timestamps, and tests:

```rust
scaffold.generate_model("User", &ModelOptions {
    fields: vec![
        ("name", "String"),
        ("email", "String"),
        ("active", "bool"),
    ],
    with_migration: true,
    with_factory: false,
}).await?;
```

**Generated Output:**

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(name: String, email: String, active: bool) -> Self {
        Self {
            id: 0,
            name,
            email,
            active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
```

### Controller Generator

Generate simple or resource controllers:

```rust
// Simple controller
scaffold.generate_controller("UserController", false).await?;

// Resource controller (CRUD operations)
scaffold.generate_controller("PostController", true).await?;
```

### Migration Generator

Generate database migrations:

```rust
scaffold.generate_migration("create_users_table").await?;
```

### Service Generator

Generate service classes:

```rust
scaffold.generate_service("AuthenticationService").await?;
```

## Naming Convention Utilities

rf-scaffold includes powerful naming convention utilities:

```rust
let naming = scaffold.naming();

// Convert between cases
naming.to_pascal_case("user_controller"); // "UserController"
naming.to_snake_case("UserController");   // "user_controller"
naming.to_kebab_case("UserService");      // "user-service"

// Pluralization
naming.pluralize("user");     // "users"
naming.singularize("users");  // "user"

// Extract base names
naming.extract_base("UserController"); // "User"
```

### Supported Conversions

- **PascalCase**: `UserController`
- **snake_case**: `user_controller`
- **kebab-case**: `user-controller`
- **Pluralization**: Handles regular and irregular English plurals

## Custom Templates

Register custom templates for project-specific code generation:

```rust
scaffold.register_template(
    "custom-struct",
    r#"pub struct {{name}} {
    pub id: u64,
    pub data: String,
}

impl {{name}} {
    pub fn new(data: String) -> Self {
        Self { id: 0, data }
    }
}"#
).await?;
```

## Project Scaffolding

Create complete project structures:

```rust
use rf_scaffold::{ProjectOptions, ProjectType};

let scaffolder = ProjectScaffolder::new(&scaffold);

scaffolder.create(&ProjectOptions {
    name: "my-api",
    project_type: ProjectType::Api,
    with_auth: true,
    with_database: true,
}).await?;
```

**Project Types:**

- `ProjectType::Api` - REST API project
- `ProjectType::FullStack` - Full-stack web application
- `ProjectType::Microservice` - Microservice architecture
- `ProjectType::Cli` - Command-line application

## Architecture

rf-scaffold is built with modularity and extensibility in mind:

- **lib.rs** (424 LOC): Core scaffold engine and API
- **generators.rs** (398 LOC): Code generators for different components
- **templates.rs** (455 LOC): Built-in Handlebars templates
- **project.rs** (429 LOC): Project scaffolding
- **naming.rs** (328 LOC): Naming convention utilities

**Total:** 2,034 lines of code with 19 comprehensive tests (100% pass rate).

## Testing

Run the test suite:

```bash
cargo test -p rf-scaffold
```

All 19 tests pass with 100% success rate, covering:

- Naming conventions (7 tests)
- Template rendering (2 tests)
- Code generators (4 tests)
- Project scaffolding (2 tests)
- File operations (3 tests)
- Engine initialization (1 test)

## Example

See `examples/scaffold-demo` for a complete working example:

```bash
cargo run -p scaffold-demo
```

This will generate:
- User model with fields
- Controllers (simple and resource)
- Service classes
- Migrations with timestamps
- Demonstrate naming utilities

## License

MIT OR Apache-2.0
