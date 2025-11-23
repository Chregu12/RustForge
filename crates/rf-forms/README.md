# Foundry Forms

HTML Form Builder & Comprehensive Validation for Foundry Core.

## Features

- **Fluent Builder API**: Chainable form building
- **Multiple Themes**: Bootstrap, Tailwind, Plain HTML
- **Comprehensive Validation**: 27+ built-in rules (Laravel-style)
- **FormRequest Pattern**: Automatic validation and authorization
- **Custom Rules Support**: Create domain-specific validation rules
- **CSRF Protection**: Automatic token generation
- **Error Display**: Styled error messages with custom messaging
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

## Validation System

### Quick Example

```rust
use foundry_forms::validation::*;
use std::collections::HashMap;

let mut data = HashMap::new();
data.insert("email".to_string(), "user@example.com".to_string());
data.insert("age".to_string(), "25".to_string());

let result = Validator::new(ValidationData::from(data))
    .rule("email", vec![required(), email()])
    .rule("age", vec![required(), numeric(), min(18.0)])
    .validate();

match result {
    Ok(_) => println!("Valid!"),
    Err(errors) => {
        for (field, messages) in errors.errors {
            println!("{}: {:?}", field, messages);
        }
    }
}
```

### 27+ Built-in Validation Rules

**Required Rules**: required, required_if, required_with

**String Rules**: email, url, alpha, alpha_numeric, regex

**Length Rules**: min_length, max_length, between, size

**Numeric Rules**: numeric, integer, min, max

**Type Rules**: string, boolean, array

**Format Rules**: ip, uuid

**Comparison Rules**: confirmed, same, different

**List Rules**: in_list, not_in

**Date Rules**: date, before, after

See [VALIDATION_GUIDE.md](./VALIDATION_GUIDE.md) for complete documentation.

### FormRequest Pattern

```rust
use foundry_forms::form_request::*;
use foundry_forms::validation::*;

struct CreateUserRequest;

impl FormRequest for CreateUserRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required(), min_length(3)])
            .rule("email", vec![required(), email()])
            .rule("password", vec![required(), min_length(8), confirmed()])
    }

    fn authorize(&self) -> bool {
        true // Add authorization logic
    }

    fn messages(&self) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("email".to_string(), "Valid email required".to_string());
        m
    }
}

// Use it
let request = CreateUserRequest;
match request.validate(data) {
    Ok(validated) => { /* Process data */ }
    Err(err) => { /* Handle errors */ }
}
```

### Custom Validation Rules

```rust
use foundry_forms::validation::*;

// Simple custom rule
let custom = custom("no_admin", |_field, value, _data| {
    if let Some(v) = value {
        if v.contains("admin") {
            return Err("Cannot contain 'admin'".to_string());
        }
    }
    Ok(())
});

Validator::new(data)
    .rule("username", vec![required(), custom])
    .validate()

// Or use the macro
use foundry_forms::custom_rule;

let uppercase = custom_rule!("uppercase", |field, value, data| {
    if let Some(v) = value {
        if v != v.to_uppercase() {
            return Err(format!("{} must be uppercase", field));
        }
    }
    Ok(())
});
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
