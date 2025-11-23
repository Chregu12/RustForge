# foundry-stub-system

A powerful and flexible stub/template system for code generation in RustForge.

## Features

- **Custom Stubs**: Override default templates with your own
- **Template Variables**: Rich variable substitution with multiple case conversions
- **Stub Publishing**: Publish default stubs to customize them
- **Stub Management**: Create, list, reset custom stubs
- **Template Engine**: Powered by Tera template engine
- **Case Conversion**: Automatic PascalCase, snake_case, kebab-case, and more

## Template Variables

All stubs have access to these variables:

- `{{ name }}` - Original name as provided
- `{{ namespace }}` - Target namespace/module path
- `{{ studly }}` - PascalCase version (e.g., UserProfile)
- `{{ snake }}` - snake_case version (e.g., user_profile)
- `{{ kebab }}` - kebab-case version (e.g., user-profile)
- `{{ camel }}` - camelCase version (e.g., userProfile)
- `{{ plural }}` - Plural form (e.g., users)
- `{{ singular }}` - Singular form (e.g., user)
- `{{ snake_plural }}` - snake_case plural (e.g., user_profiles)
- `{{ studly_plural }}` - PascalCase plural (e.g., UserProfiles)
- Custom variables via context

## Usage

### Basic Stub Rendering

```rust
use foundry_stub_system::{StubManager, StubContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = StubManager::new("./stubs");

    let context = StubContext::new("User")
        .with_namespace("app::models")
        .with_property("name", "String")
        .with_property("email", "String");

    let rendered = manager.render("model", context).await?;
    println!("{}", rendered);

    Ok(())
}
```

### Using Case Converters

```rust
use foundry_stub_system::CaseConverter;

let name = "UserProfile";

println!("Studly: {}", CaseConverter::studly(name));   // UserProfile
println!("Snake: {}", CaseConverter::snake(name));     // user_profile
println!("Kebab: {}", CaseConverter::kebab(name));     // user-profile
println!("Camel: {}", CaseConverter::camel(name));     // userProfile
println!("Plural: {}", CaseConverter::plural("user")); // users
```

### Working with Variables

```rust
use foundry_stub_system::StubVariables;

let vars = StubVariables::new("BlogPost")
    .with_namespace("app::models")
    .with_custom("table", "blog_posts")
    .with_custom("author", "system");

let context = vars.to_context();
// Use context for template rendering
```

### Publishing Stubs

```rust
use foundry_stub_system::StubPublisher;

let publisher = StubPublisher::new("./stubs");

// Publish all default stubs
let published = publisher.publish_all().await?;
println!("Published: {:?}", published);

// Publish specific stub
publisher.publish("model").await?;

// List published stubs
let stubs = publisher.list_published().await?;
println!("Custom stubs: {:?}", stubs);

// Reset to default
publisher.reset("model").await?;
```

### Creating Custom Stubs

```rust
use foundry_stub_system::StubPublisher;

let publisher = StubPublisher::new("./stubs");

let custom_stub = r#"
pub struct {{ studly }} {
    pub id: i64,
    pub {{ snake }}_name: String,
}

impl {{ studly }} {
    pub fn new() -> Self {
        Self {
            id: 0,
            {{ snake }}_name: String::new(),
        }
    }
}
"#;

publisher.create_custom("my_model", custom_stub).await?;
```

### Loading Custom Stubs

```rust
use foundry_stub_system::StubManager;

let mut manager = StubManager::new("./stubs");

// Load all custom stubs from ./stubs directory
manager.load_custom_stubs().await?;

// Custom stubs take priority over defaults
let stub = manager.get_stub("model")?;
if stub.is_custom {
    println!("Using custom model stub");
}
```

## Default Stubs

### Model Stub

```rust
use serde::{Deserialize, Serialize};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{{ snake_plural }}")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
{{ properties }}
    pub created_at: DateTime,
    pub updated_at: DateTime,
}
```

### Controller Stub

```rust
use axum::{extract::Path, response::Json};

pub struct {{ studly }}Controller;

impl {{ studly }}Controller {
    pub async fn index() -> Result<Json<Vec<{{ studly }}>>, AppError> {
        // List all {{ plural }}
        todo!("Implement index")
    }

    pub async fn show(Path(id): Path<i64>) -> Result<Json<{{ studly }}>, AppError> {
        // Show single {{ singular }}
        todo!("Implement show")
    }
}
```

### Service Stub

```rust
pub struct {{ studly }}Service;

impl {{ studly }}Service {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_all(&self) -> Result<Vec<{{ studly }}>, AppError> {
        todo!("Implement get_all")
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<{{ studly }}>, AppError> {
        todo!("Implement get_by_id")
    }
}
```

## Advanced Usage

### Stub Management

```rust
use foundry_stub_system::StubManager;

let manager = StubManager::new("./stubs");

// List all available stubs
let stubs = manager.list_stubs();
for stub in stubs {
    println!("- {}", stub);
}

// Check if custom stub exists
if manager.has_custom_stub("model") {
    println!("Custom model stub found");
}
```

### Context Merging

```rust
use foundry_stub_system::StubContext;

let base_context = StubContext::new("User")
    .with_namespace("app::models");

let extended_context = StubContext::new("User")
    .with_property("email", "String")
    .with_custom("table", "users");

let merged = base_context.merge(extended_context);
```

### Custom Properties

```rust
use foundry_stub_system::StubContext;

let context = StubContext::new("Product")
    .with_property("id", "i64")
    .with_property("name", "String")
    .with_property("price", "Decimal")
    .with_property("stock", "i32");

// Properties are rendered in the {{ properties }} placeholder
let rendered = manager.render("model", context).await?;
```

## CLI Integration

The stub system is designed to integrate with `make:*` commands:

```bash
# Use default stub
foundry make:model User

# After publishing, customize the stub
foundry stub:publish model
# Edit stubs/model.stub

# Use customized stub
foundry make:model Post
# Now uses your custom stub
```

## Stub Directory Structure

```
project/
├── stubs/
│   ├── model.stub          # Custom model template
│   ├── controller.stub     # Custom controller template
│   ├── service.stub        # Custom service template
│   ├── migration.stub      # Custom migration template
│   └── my_custom.stub      # Your custom stub type
```

## Testing

Run tests with:

```bash
cargo test --package foundry-stub-system
```

## Example: Complete Workflow

```rust
use foundry_stub_system::{StubManager, StubPublisher, StubContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup
    let publisher = StubPublisher::new("./stubs");
    let mut manager = StubManager::new("./stubs");

    // 2. Publish default stubs to customize
    publisher.publish_all().await?;

    // 3. Load custom stubs
    manager.load_custom_stubs().await?;

    // 4. Create context
    let context = StubContext::new("BlogPost")
        .with_namespace("app::models")
        .with_property("title", "String")
        .with_property("content", "Text")
        .with_property("author_id", "i64")
        .with_custom("table", "blog_posts");

    // 5. Render
    let model_code = manager.render("model", context.clone()).await?;
    let controller_code = manager.render("controller", context.clone()).await?;

    // 6. Write to files
    tokio::fs::write("src/models/blog_post.rs", model_code).await?;
    tokio::fs::write("src/controllers/blog_post_controller.rs", controller_code).await?;

    println!("Generated BlogPost model and controller!");

    Ok(())
}
```

## Tips

1. **Always load custom stubs** before rendering to ensure they take priority
2. **Use meaningful variable names** in your custom stubs
3. **Test your stubs** before using them in production code generation
4. **Version control your stubs** directory for team consistency
5. **Document custom variables** in your stub files with comments

## License

MIT OR Apache-2.0
