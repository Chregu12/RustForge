# rf-validation-derive

Procedural macros for the `rf-validation` crate.

This crate provides the `#[derive(Validate)]` macro that automatically generates validation implementations based on field attributes.

## Features

- **Declarative Validation**: Use attributes to define validation rules
- **Type-Safe**: Compile-time validation rule checking
- **30+ Built-in Rules**: String, number, date, and database validations
- **Nested Validation**: Automatically validate nested structs
- **Optional Fields**: Smart handling of `Option<T>` types
- **Custom Messages**: Override default error messages
- **Multiple Rules**: Combine multiple validation rules per field

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rf-validation = { version = "0.1", features = ["derive"] }
```

## Quick Start

```rust
use rf_validation_derive::Validate;
use validator::Validate as ValidatorValidate;

#[derive(Validate)]
struct CreateUser {
    #[validate(required, email, max = 255)]
    email: String,

    #[validate(required, min = 8, max = 128)]
    password: String,

    #[validate(required, min = 2, max = 100)]
    name: String,
}

fn main() {
    let user = CreateUser {
        email: "user@example.com".to_string(),
        password: "securepassword".to_string(),
        name: "John Doe".to_string(),
    };

    match user.validate() {
        Ok(_) => println!("User is valid!"),
        Err(e) => println!("Validation failed: {:?}", e),
    }
}
```

## Supported Validation Rules

### String Rules

- `required` - Field must not be empty
- `string` - Field must be a valid string (type-level)
- `email` - Valid email address
- `url` - Valid URL
- `ip` - Valid IP address (v4 or v6)
- `uuid` - Valid UUID
- `min = n` - Minimum length
- `max = n` - Maximum length
- `between = [min, max]` - Length between min and max
- `starts_with = "prefix"` - Must start with prefix
- `ends_with = "suffix"` - Must end with suffix
- `regex = "pattern"` - Must match regex pattern
- `alpha` - Only alphabetic characters
- `alpha_numeric` - Only alphanumeric characters
- `lowercase` - Only lowercase characters
- `uppercase` - Only uppercase characters

### Number Rules

- `integer` - Must be an integer
- `numeric` - Must be numeric
- `digits = n` - Must have exactly n digits
- `digits_between = [min, max]` - Digits between min and max
- `positive` - Must be positive
- `negative` - Must be negative

### Date Rules

- `date` - Must be a valid date
- `date_format = "format"` - Must match date format
- `before = "date"` - Must be before date
- `after = "date"` - Must be after date
- `between_dates = ["date1", "date2"]` - Must be between dates

### Database Rules

- `exists = "table"` - Value must exist in table
- `exists = ["table", "column"]` - Value must exist in table.column
- `unique = "table"` - Value must be unique in table
- `unique = ["table", "column"]` - Value must be unique in table.column
- `unique_ignore = ["table", "column", id]` - Unique, ignoring specific id

### Conditional Rules

- `required_if = ["field", "value"]` - Required if field equals value
- `required_unless = ["field", "value"]` - Required unless field equals value
- `required_with = "field"` - Required if field is present

## Optional Fields

The derive macro intelligently handles `Option<T>` types:

```rust
#[derive(Validate)]
struct UpdateUser {
    // Optional field - validates only if Some
    #[validate(email)]
    email: Option<String>,

    // Required optional field - must be Some
    #[validate(required)]
    name: Option<String>,
}
```

## Nested Validation

Validate nested structs and collections:

```rust
#[derive(Validate)]
struct Tag {
    #[validate(required, min = 2)]
    name: String,
}

#[derive(Validate)]
struct Post {
    #[validate(required)]
    title: String,

    // Automatically validates each tag
    #[validate]
    tags: Vec<Tag>,
}
```

## Custom Messages

Override default error messages:

```rust
#[derive(Validate)]
struct CreatePost {
    #[validate(required, message = "Title is required")]
    #[validate(max = 255, message = "Title is too long")]
    title: String,
}
```

## Multiple Rules

Combine multiple rules per field:

```rust
#[derive(Validate)]
struct Registration {
    #[validate(required, email, max = 255)]
    email: String,

    #[validate(required, min = 8, max = 128, alpha_numeric)]
    username: String,
}
```

## Examples

See the `examples/` directory for comprehensive examples:

- `complete_demo.rs` - Demonstrates all validation features

Run examples:

```bash
cargo run --example complete_demo
```

## Testing

Run the test suite:

```bash
cargo test
```

## Integration with Axum

Use with the `ValidatedJson` extractor from `rf-validation`:

```rust
use rf_validation::{ValidatedJson, Validate};
use axum::{routing::post, Router};

#[derive(Validate, serde::Deserialize)]
struct CreateUser {
    #[validate(email)]
    email: String,
}

async fn create_user(
    ValidatedJson(user): ValidatedJson<CreateUser>,
) -> String {
    format!("Created user: {}", user.email)
}

let app = Router::new().route("/users", post(create_user));
```

## How It Works

The `#[derive(Validate)]` macro generates an implementation of the `Validate` trait from the `validator` crate. It parses the `#[validate(...)]` attributes on each field and generates the corresponding validation code.

For example:

```rust
#[derive(Validate)]
struct CreateUser {
    #[validate(required, email, max = 255)]
    email: String,
}
```

Expands to:

```rust
impl Validate for CreateUser {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        // Required check
        if self.email.is_empty() {
            errors.add("email", /* error */);
        }

        // Email validation
        if !rf_validation::validators::email::validate_email(&self.email) {
            errors.add("email", /* error */);
        }

        // Max length check
        if self.email.len() > 255 {
            errors.add("email", /* error */);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
```

## License

MIT OR Apache-2.0
