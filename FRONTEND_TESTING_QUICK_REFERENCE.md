# Frontend & Testing Features - Quick Reference

This is a condensed quick reference for the newly implemented frontend and testing features.
For comprehensive documentation, see [FRONTEND_TESTING_IMPLEMENTATION.md](FRONTEND_TESTING_IMPLEMENTATION.md).

---

## Blade Components

### Basic Usage
```rust
use rf_blade::components::*;

let mut registry = ComponentRegistry::new();
registry.register("alert", AlertComponent::new())?;
let compiler = ComponentCompiler::new(Arc::new(registry))?;
let html = compiler.compile(r#"<x-alert>Message</x-alert>"#)?;
```

### Named Slots
```rust
r#"
<x-card>
    <x-slot name="header">Title</x-slot>
    Body
</x-card>
"#
```

---

## View Composers

### Global Composer
```rust
use rf_views::composers;

composers::composer_fn("*", |_, context| {
    context.insert("app_name", "MyApp");
    Ok(())
})?;
```

### Pattern Composer
```rust
composers::composer_fn("posts.*", |_, context| {
    context.insert("categories", get_categories());
    Ok(())
})?;
```

---

## Database Seeders

### Basic Seeder
```rust
use rf_testing::seeder::*;

#[async_trait]
impl Seeder for UserSeeder {
    fn name(&self) -> &str { "UserSeeder" }
    async fn run(&self) -> Result<(), SeederError> {
        // Seed logic
        Ok(())
    }
}
```

### Run with Production Guard
```rust
DatabaseSeeder::new()
    .add(UserSeeder)
    .run_all().await?;
```

---

## Factory Features

### Sequences
```rust
use rf_testing::Sequence;

let seq = Sequence::new();
let id = seq.next();  // 0, 1, 2...
```

### States
```rust
let admin = UserFactory::new()
    .state(|u| u.role = "admin".to_string())
    .create().await?;
```

### Relationships
```rust
let post = PostFactory::new()
    .for_user(&user)
    .create().await?;
```

---

## Examples

Run comprehensive examples:

```bash
cargo run --example full_component_system --package rf-blade
cargo run --example view_composers_example --package rf-views
cargo run --example advanced_factories_example --package rf-testing
cargo run --example database_seeders_example --package rf-testing
```

---

For complete documentation, see:
- [Implementation Guide](FRONTEND_TESTING_IMPLEMENTATION.md)
- [Implementation Summary](IMPLEMENTATION_SUMMARY.md)
- Example files in `/crates/*/examples/`
