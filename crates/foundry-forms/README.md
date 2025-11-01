# Foundry Forms

HTML Form Builder & Validation for Foundry Core.

## Features

- **Fluent Builder API**: Chainable form building
- **Multiple Themes**: Bootstrap, Tailwind, Plain HTML
- **Built-in Validation**: Required, email, min/max length, patterns
- **CSRF Protection**: Automatic token generation
- **Error Display**: Styled error messages
- **Field Types**: Text, email, password, textarea, select, checkbox, file

## Quick Start

```rust
use foundry_forms::{Form, Field, Theme};

let form = Form::new("user_form")
    .action("/users")
    .field(
        Field::text("name")
            .label("Name")
            .placeholder("Enter your name")
            .required()
            .build()
    )
    .field(
        Field::email("email")
            .label("Email")
            .placeholder("your@email.com")
            .required()
            .build()
    )
    .field(
        Field::password("password")
            .label("Password")
            .min_length(8)
            .build()
    )
    .submit("Create User")
    .build();

let html = form.render(Theme::Tailwind)?;
```

## CLI Commands

```bash
# Generate form builder
foundry make:form ContactForm
```

## Field Types

```rust
// Text input
Field::text("name").label("Name").required()

// Email with validation
Field::email("email").label("Email").required()

// Password with minimum length
Field::password("password").label("Password").min_length(8)

// Number input
Field::number("age").label("Age")

// Textarea
Field::textarea("message").label("Message").build()

// Select dropdown
Field::select("country", vec![
    SelectOption { value: "us".to_string(), label: "United States".to_string() },
    SelectOption { value: "uk".to_string(), label: "United Kingdom".to_string() },
])

// Checkbox
Field::checkbox("agree").label("I agree to terms")

// File upload
Field::file("avatar").label("Profile Picture")
```

## Validation

```rust
use foundry_forms::{FormData, ValidationRule};

// Validate form data
let mut data = FormData::new();
data.insert("email", "test@example.com");
data.insert("password", "secure123");

match form.validate(&data) {
    Ok(_) => println!("Valid!"),
    Err(errors) => {
        for (field, messages) in errors.errors {
            println!("{}: {:?}", field, messages);
        }
    }
}
```

## Custom Validation Rules

```rust
Field::text("username")
    .label("Username")
    .min_length(3)
    .max_length(20)
    .pattern(r"^[a-zA-Z0-9_]+$".to_string())
    .build()
```

## Themes

```rust
// Tailwind CSS
form.render(Theme::Tailwind)?

// Bootstrap 5
form.render(Theme::Bootstrap)?

// Plain HTML
form.render(Theme::Plain)?
```

## CSRF Protection

```rust
use foundry_forms::CsrfProtection;

let csrf = CsrfProtection::new(3600); // 1 hour TTL
let token = csrf.generate("session_id");

// Validate
if csrf.validate("session_id", &token) {
    println!("Valid CSRF token");
}
```

## Rendering with Data & Errors

```rust
let data = FormData::new();
let errors = FormErrors::new();

let html = form.render_with_data(Theme::Tailwind, &data, &errors)?;
```
