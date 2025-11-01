# RustForge Stub Customization Guide

This guide explains how to customize code generation stubs in RustForge, allowing you to tailor the output of `make:*` commands to your project's needs.

## Overview

The stub customization system provides:
- **Built-in stubs** for common code generation tasks
- **Custom stubs** to override built-in templates
- **Variable interpolation** for dynamic content
- **Stub publishing** to extract stubs for customization
- **Flexible stub paths** for organization

## Built-in Stubs

RustForge comes with stubs for common patterns:

| Stub | ID | Purpose |
|------|----|----|
| Model | `model` | Eloquent-style model class |
| Controller | `controller` | HTTP controller class |
| Migration | `migration` | Database migration |
| Job | `job` | Queueable job class |

### Example: Built-in Model Stub

```php
<?php

namespace {{namespace}};

use Illuminate\Database\Eloquent\Model;

class {{name}} extends Model
{
    protected $table = '{{table}}';

    protected $fillable = [
        {{fillable}},
    ];
}
```

## Publishing Stubs

### Publishing All Stubs

Publish all available stubs to customize them:

```bash
foundry vendor:publish --tag=stubs
```

This creates a `stubs/` directory in your project root with copies of all built-in stubs.

### Publishing Specific Stubs

Publish only specific stubs:

```bash
foundry vendor:publish --tag=model_stubs
foundry vendor:publish --tag=controller_stubs
```

### Force Overwrite

Overwrite existing published stubs:

```bash
foundry vendor:publish --tag=stubs --force
```

## Available Variables

All stubs support these variables:

### Common Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{{namespace}}` | Class namespace | `App\Models` |
| `{{name}}` | Class name | `User` |
| `{{command}}` | Command name | `make:model` |
| `{{timestamp}}` | Current timestamp (RFC3339) | `2024-11-01T10:30:00+00:00` |
| `{{year}}` | Current year | `2024` |

### Model Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{{table}}` | Table name | `users` |
| `{{fillable}}` | Fillable fields | `'name', 'email'` |

### Migration Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{{table}}` | Table name | `users` |

### Custom Variables

Define custom variables when generating code:

```bash
foundry make:model User \
  --table=users \
  --fillable="name,email,password"
```

## Using Custom Stubs

### Step 1: Publish Stubs

```bash
foundry vendor:publish --tag=stubs
```

### Step 2: Customize

Edit `stubs/model.php`:

```php
<?php

namespace {{namespace}};

use Illuminate\Database\Eloquent\Model;
use App\Traits\Timestampable;

class {{name}} extends Model
{
    use Timestampable;

    protected $table = '{{table}}';

    protected $fillable = [
        {{fillable}},
    ];

    // Custom relationship methods
    public function related()
    {
        // Custom implementation
    }
}
```

### Step 3: Use

The `make:model` command now uses your custom stub:

```bash
foundry make:model Product --table=products --fillable="name,price,description"
```

Generated file:

```php
<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use App\Traits\Timestampable;

class Product extends Model
{
    use Timestampable;

    protected $table = 'products';

    protected $fillable = [
        'name', 'price', 'description',
    ];

    // Custom relationship methods
    public function related()
    {
        // Custom implementation
    }
}
```

## Programmatic API

### Using StubManager

```rust
use foundry_api::{StubManager, StubVariables};

// Create manager
let mut manager = StubManager::new("stubs");

// Register built-in stubs
let model = Stub::new(
    "model",
    "Model",
    "class {{name}} { ... }",
    "rs"
);
manager.register(model);

// Get and render
let stub = manager.get("model")?;
let mut vars = StubVariables::new();
vars.set("name", "User");

let rendered = stub.render(&vars)?;
println!("{}", rendered);
```

### Loading from Filesystem

```rust
use foundry_api::StubManager;

let mut manager = StubManager::new("stubs");

// Load stubs from filesystem
manager.load_from_filesystem()?;

// List available stubs
for stub_id in manager.list() {
    println!("Available stub: {}", stub_id);
}
```

### Publishing Stubs

```rust
use foundry_api::{StubManager, StubPublisher, PublishConfig};

let manager = StubManager::new("stubs");
let config = PublishConfig::new("stubs", "app/stubs").force(true);
let publisher = StubPublisher::new(manager, config);

// Publish stubs
let published = publisher.publish()?;

for pub_stub in published {
    println!("Published: {} -> {}",
        pub_stub.id,
        pub_stub.destination.display()
    );
}
```

### Preview Before Publishing

```rust
let publisher = StubPublisher::new(manager, config);

// Preview what will be published
let stubs = publisher.preview()?;

for stub in stubs {
    println!("Will publish: {}", stub.relative_path());
    println!("Size: {} bytes", stub.size);
}
```

## Advanced Patterns

### Creating Custom Stubs

Create a new file `stubs/repository.php`:

```php
<?php

namespace {{namespace}};

interface {{name}} extends Repository
{
    // Repository methods
}
```

### Organizing Stubs by Category

Stubs are organized by prefix:

```
stubs/
├── model.php           (model_stubs category)
├── model_factory.php   (model_stubs category)
├── controller.php      (controller_stubs category)
└── command.php         (command_stubs category)
```

Get stubs by category:

```rust
let model_stubs = manager.by_category("model");
```

### Conditional Stub Content

Use conditional logic in stubs:

```php
<?php

namespace {{namespace}};

class {{name}} {{#extends}}extends {{extends}}{{/extends}}
{
    {{#implements}}implements {{implements}}{{/implements}}

    {{#traits}}use {{traits}};{{/traits}}

    // Class content
}
```

### Multiple Stub Paths

Configure multiple stub directories:

```rust
let mut manager = StubManager::new("stubs");
manager.add_path("vendor/custom-stubs");
manager.add_path("app/custom-stubs");

// Load stubs from all paths (later paths override earlier ones)
manager.load_from_filesystem()?;
```

## In Command Context

### Render Stub in Command

```rust
use foundry_api::{Stub, StubVariables, Console};
use foundry_plugins::{CommandContext, CommandResult, CommandError};

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let console = Console::new(ctx.verbosity.level());

    // Create variables
    let mut vars = StubVariables::new();
    vars.set("namespace", "App\\Models");
    vars.set("name", "User");
    vars.set("table", "users");
    vars.with_common_vars("make:model");

    // Get and render stub
    let stub = manager.get("model")?;
    let code = stub.render(&vars)?;

    // Write to file
    fs::write("app/Models/User.php", &code)?;

    console.success("Model created successfully");
    Ok(CommandResult::success("Model created"))
}
```

## Best Practices

1. **Keep stubs simple and focused**
   - One concern per stub
   - Avoid complex logic

2. **Use meaningful variable names**
   ```php
   {{namespace}}, {{name}}, {{table}}  // Good
   {{n}}, {{ns}}, {{t}}                // Avoid
   ```

3. **Document custom variables**
   ```bash
   # Usage: make:model NAME --table=TABLE --fillable=FIELDS
   ```

4. **Version control your stubs**
   ```bash
   git add stubs/
   git commit -m "Add custom code generation stubs"
   ```

5. **Test stub rendering**
   ```rust
   #[test]
   fn test_model_stub_rendering() {
       let stub = manager.get("model").unwrap();
       let mut vars = StubVariables::new();
       vars.set("namespace", "App");
       vars.set("name", "Test");

       let result = stub.render(&vars).unwrap();
       assert!(result.contains("class Test"));
   }
   ```

## Troubleshooting

### Stubs Not Found

Ensure stubs directory exists and is readable:

```bash
ls -la stubs/
```

### Variables Not Replaced

Check that variable names match exactly (case-sensitive):

```php
// Correct
{{namespace}}, {{Name}}, {{TABLE}}

// Incorrect
{{Namespace}}, {{name}}, {{table}}
```

### Publish Fails with "File Exists"

Use `--force` flag to overwrite:

```bash
foundry vendor:publish --tag=stubs --force
```

## Migration from Laravel

| Laravel | RustForge |
|---------|-----------|
| `stubs/` directory | `stubs/` directory |
| Variable syntax `{{ }}` | Variable syntax `{{ }}` |
| `php artisan make:model` | `foundry make:model` |
| `vendor/publish --tag=stubs` | `vendor:publish --tag=stubs` |
| Custom stubs loaded automatically | Published stubs loaded automatically |

## See Also

- [RustForge Documentation](https://github.com/Chregu12/RustForge)
- [Programmatic Execution Guide](PROGRAMMATIC_EXECUTION_GUIDE.md)
- [Advanced Input Handling Guide](ADVANCED_INPUT_HANDLING_GUIDE.md)
- [Verbosity Levels Guide](VERBOSITY_LEVELS_GUIDE.md)
