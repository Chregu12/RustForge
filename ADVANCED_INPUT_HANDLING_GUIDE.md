# RustForge Advanced Input Handling Guide

This guide covers the advanced input parsing and validation system for RustForge commands.

## Overview

RustForge provides robust input handling with support for:
- **Positional arguments**
- **Named options** with single and multiple values
- **Boolean flags**
- **Option arrays** (multiple values for same option)
- **Input validation** with rules
- **Type-safe parsing**

## Basic Usage

### Parsing Arguments and Options

```rust
use foundry_api::input::InputParser;

// Parse command line arguments
let args = vec![
    "create".to_string(),
    "--name=John".to_string(),
    "--age".to_string(),
    "30".to_string(),
    "--admin".to_string(),
];

let parser = InputParser::from_args(&args);

// Get positional argument
let command = parser.first_argument();
assert_eq!(command, Some("create".to_string()));

// Get single option
let name = parser.option("name");
assert_eq!(name, Some("John".to_string()));

// Get option with default
let city = parser.option_with_default("city", "New York");

// Check for flags
if parser.has_flag("admin") {
    println!("Admin mode enabled");
}
```

### Option Syntax Variations

The parser supports multiple option syntax styles:

```rust
use foundry_api::input::InputParser;

// All of these are equivalent:

// Style 1: --name=value
let args = vec!["--name=John".to_string()];
let parser = InputParser::from_args(&args);
assert_eq!(parser.option("name"), Some("John".to_string()));

// Style 2: --name value
let args = vec!["--name".to_string(), "John".to_string()];
let parser = InputParser::from_args(&args);
assert_eq!(parser.option("name"), Some("John".to_string()));

// Style 3: Short flags
let args = vec!["-n".to_string(), "John".to_string()];
let parser = InputParser::from_args(&args);
assert_eq!(parser.option("n"), Some("John".to_string()));
```

## Option Arrays

Handle multiple values for the same option:

```rust
use foundry_api::input::InputParser;

// Multiple --tag options
let args = vec![
    "--tag".to_string(),
    "admin".to_string(),
    "--tag".to_string(),
    "user".to_string(),
    "--tag".to_string(),
    "moderator".to_string(),
];

let parser = InputParser::from_args(&args);

// Get all values
let tags = parser.option_array("tag");
assert_eq!(tags, vec!["admin".to_string(), "user".to_string(), "moderator".to_string()]);
```

### Use in Commands

```rust
use foundry_api::input::InputParser;
use foundry_plugins::{CommandContext, CommandResult, CommandError};

async fn execute(ctx: CommandContext) -> Result<CommandResult, CommandError> {
    let parser = InputParser::from_args(&ctx.args);

    // Get multiple tags
    let tags = parser.option_array("tags");
    println!("Tags: {:?}", tags);

    // Get multiple email addresses
    let emails = parser.option_array("email");
    for email in emails {
        println!("Sending to: {}", email);
    }

    Ok(CommandResult::success("Processed successfully"))
}
```

## Input Validation

### Basic Validation

```rust
use foundry_api::input::{InputParser, InputValidator};

let args = vec!["--name".to_string(), "John".to_string()];
let parser = InputParser::from_args(&args);

let validator = InputValidator::new()
    .required("name")
    .required("email");

match validator.validate(&parser) {
    Ok(()) => println!("Input is valid"),
    Err(violations) => {
        for violation in violations {
            println!("Field {}: {}", violation.field, violation.message);
        }
    }
}
```

### Validation Rules

```rust
use foundry_api::input::{InputParser, InputValidator, Rule};

let args = vec!["--username".to_string(), "john_doe".to_string()];
let parser = InputParser::from_args(&args);

let validator = InputValidator::new()
    .rule("username", vec![
        Rule::Required,
        Rule::MinLength(3),
        Rule::MaxLength(20),
    ]);

validator.validate(&parser)?;
```

### Common Validation Patterns

```rust
use foundry_api::input::{InputParser, InputValidator, Rule};

let args = vec!["--status".to_string(), "active".to_string()];
let parser = InputParser::from_args(&args);

// Enum-like validation: must be one of predefined values
let validator = InputValidator::new()
    .rule("status", vec![
        Rule::Required,
        Rule::OneOf(vec![
            "active".to_string(),
            "inactive".to_string(),
            "pending".to_string(),
        ]),
    ]);

validator.validate(&parser)?;
```

## In Command Context

### Extract Input Information

```rust
use foundry_api::input::InputParser;
use foundry_plugins::CommandContext;

async fn process_command(ctx: CommandContext) {
    let parser = InputParser::from_args(&ctx.args);

    // Get all positional arguments
    let args = parser.arguments();
    println!("Arguments: {:?}", args);

    // Get all option names
    let options = parser.option_names();
    println!("Options provided: {:?}", options);

    // Get all flags
    let flags = parser.flags();
    println!("Flags: {:?}", flags);
}
```

### Full Command Example

```rust
use foundry_api::input::{InputParser, InputValidator, Rule};
use foundry_plugins::{CommandContext, CommandResult, CommandError, FoundryCommand, CommandDescriptor};
use async_trait::async_trait;
use std::sync::Arc;

pub struct CreateUserCommand;

#[async_trait]
impl FoundryCommand for CreateUserCommand {
    fn descriptor(&self) -> &CommandDescriptor {
        // ... descriptor implementation
    }

    async fn execute(&self, ctx: CommandContext) -> Result<CommandResult, CommandError> {
        let parser = InputParser::from_args(&ctx.args);

        // Validate input
        let validator = InputValidator::new()
            .required("email")
            .string_length("name", 2, 50);

        validator.validate(&parser)
            .map_err(|violations| {
                let msg = violations
                    .iter()
                    .map(|v| format!("{}: {}", v.field, v.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                CommandError::Message(msg)
            })?;

        // Extract values
        let email = parser.option("email").unwrap();
        let name = parser.option("name");
        let roles = parser.option_array("role");
        let is_admin = parser.has_flag("admin");

        // Process command
        println!("Creating user: {}", email);
        if let Some(n) = name {
            println!("Name: {}", n);
        }
        if !roles.is_empty() {
            println!("Roles: {:?}", roles);
        }
        if is_admin {
            println!("Admin privileges granted");
        }

        Ok(CommandResult::success("User created successfully"))
    }
}
```

## Advanced Patterns

### Optional Arguments with Defaults

```rust
use foundry_api::input::InputParser;

let args = vec!["--output".to_string(), "file.txt".to_string()];
let parser = InputParser::from_args(&args);

// Use default if not provided
let output_file = parser.option_with_default("output", "stdout");
let format = parser.option_with_default("format", "json");

println!("Output: {}", output_file);
println!("Format: {}", format);
```

### Conditional Argument Processing

```rust
use foundry_api::input::InputParser;

let args = vec![
    "--input".to_string(),
    "data.json".to_string(),
    "--format".to_string(),
    "json".to_string(),
];
let parser = InputParser::from_args(&args);

match parser.option("format").as_deref() {
    Some("json") => parse_json(&parser.option("input").unwrap()),
    Some("csv") => parse_csv(&parser.option("input").unwrap()),
    Some("yaml") => parse_yaml(&parser.option("input").unwrap()),
    _ => println!("Unsupported format"),
}
```

### Batch Processing with Array Options

```rust
use foundry_api::input::InputParser;

let args = vec![
    "--file".to_string(),
    "users.csv".to_string(),
    "--file".to_string(),
    "posts.csv".to_string(),
    "--file".to_string(),
    "comments.csv".to_string(),
];

let parser = InputParser::from_args(&args);
let files = parser.option_array("file");

for (idx, file) in files.iter().enumerate() {
    println!("[{}/{}] Processing: {}", idx + 1, files.len(), file);
    process_file(file)?;
}
```

## Error Handling

### Validation Error Details

```rust
use foundry_api::input::{InputParser, InputValidator};

let args = vec![];
let parser = InputParser::from_args(&args);

let validator = InputValidator::new()
    .required("name")
    .required("email");

match validator.validate(&parser) {
    Ok(()) => println!("Valid"),
    Err(violations) => {
        for violation in violations {
            println!(
                "Validation Error - Field: {}, Rule: {}, Message: {}",
                violation.field, violation.rule, violation.message
            );
        }
    }
}
```

## Best Practices

1. **Always validate user input**
   ```rust
   let validator = InputValidator::new().required("name");
   validator.validate(&parser)?;
   ```

2. **Provide clear error messages**
   ```rust
   let msg = violations
       .iter()
       .map(|v| format!("{}: {}", v.field, v.message))
       .collect::<Vec<_>>()
       .join("; ");
   ```

3. **Use defaults wisely**
   ```rust
   let timeout = parser.option_with_default("timeout", "30");
   ```

4. **Document expected arguments**
   ```rust
   // In your command descriptor or help text
   // Usage: command [options]
   // Options:
   //   --name <value>      User's name (required)
   //   --email <value>     Email address (required)
   //   --roles <value>     User roles (can be repeated)
   //   --admin             Grant admin privileges
   ```

5. **Handle missing required arguments**
   ```rust
   let name = parser.option("name")
       .ok_or_else(|| CommandError::Message("Name is required".to_string()))?;
   ```

## Comparison with Laravel Input Handling

| Feature | Laravel | RustForge | Status |
|---------|---------|-----------|--------|
| Positional args | ✓ | ✓ | ✅ |
| Named options | ✓ | ✓ | ✅ |
| Option arrays | ✓ | ✓ | ✅ |
| Boolean flags | ✓ | ✓ | ✅ |
| Validation | ✓ | ✓ | ✅ |
| Input defaults | ✓ | ✓ | ✅ |
| Type conversion | ✓ | ⚠️ | Partial |

## Performance Considerations

- **InputParser** is lightweight and allocation-efficient
- **InputValidator** performs validation once
- **Option arrays** stored as vectors for O(1) access
- **Flags** stored as vector with linear search (typically small number)

## See Also

- [Artisan Facade Guide](PROGRAMMATIC_EXECUTION_GUIDE.md)
- [Verbosity Levels Guide](VERBOSITY_LEVELS_GUIDE.md)
- [RustForge Documentation](https://github.com/Chregu12/RustForge)
