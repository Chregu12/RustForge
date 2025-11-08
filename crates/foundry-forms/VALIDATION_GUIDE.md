# Foundry Forms Validation Guide

Comprehensive Laravel-style validation system for Rust.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Available Validation Rules](#available-validation-rules)
3. [Validator API](#validator-api)
4. [FormRequest Pattern](#formrequest-pattern)
5. [Custom Validation Rules](#custom-validation-rules)
6. [Error Handling](#error-handling)
7. [Advanced Usage](#advanced-usage)

---

## Quick Start

```rust
use foundry_forms::validation::*;
use std::collections::HashMap;

// Create validation data
let mut data = HashMap::new();
data.insert("email".to_string(), "user@example.com".to_string());
data.insert("age".to_string(), "25".to_string());

// Validate with rules
let result = Validator::new(ValidationData::from(data))
    .rule("email", vec![required(), email()])
    .rule("age", vec![required(), numeric(), min(18.0)])
    .validate();

match result {
    Ok(validated_data) => println!("Validation passed!"),
    Err(errors) => {
        for (field, messages) in errors.errors {
            println!("{}: {:?}", field, messages);
        }
    }
}
```

---

## Available Validation Rules

### Required Rules

#### `required()`
Field must have a value.

```rust
.rule("name", vec![required()])
```

#### `required_if(field, value)`
Field is required if another field equals a specific value.

```rust
.rule("admin_code", vec![required_if("role", "admin")])
```

#### `required_with(field)`
Field is required if another field is present.

```rust
.rule("password", vec![required_with("email")])
```

---

### String Validation Rules

#### `email()`
Field must be a valid email address.

```rust
.rule("email", vec![email()])
// Valid: "user@example.com"
// Invalid: "not-an-email"
```

#### `url()`
Field must be a valid URL.

```rust
.rule("website", vec![url()])
// Valid: "https://example.com"
// Invalid: "not-a-url"
```

#### `alpha()`
Field must contain only alphabetic characters.

```rust
.rule("name", vec![alpha()])
// Valid: "JohnDoe"
// Invalid: "John123"
```

#### `alpha_numeric()`
Field must contain only alphanumeric characters.

```rust
.rule("username", vec![alpha_numeric()])
// Valid: "User123"
// Invalid: "User-123"
```

#### `regex(pattern)`
Field must match a regular expression pattern.

```rust
.rule("username", vec![regex(r"^[a-z0-9_]+$")])
// Valid: "user_123"
// Invalid: "User-123"
```

---

### Length Validation Rules

#### `min_length(n)`
Field must have at least n characters.

```rust
.rule("password", vec![min_length(8)])
```

#### `max_length(n)`
Field must not exceed n characters.

```rust
.rule("username", vec![max_length(20)])
```

#### `between(min, max)`
Field length must be between min and max.

```rust
.rule("username", vec![between(3, 20)])
```

#### `size(n)`
Field must be exactly n characters.

```rust
.rule("code", vec![size(6)])
```

---

### Numeric Validation Rules

#### `numeric()`
Field must be a valid number (integer or float).

```rust
.rule("price", vec![numeric()])
// Valid: "19.99", "42"
// Invalid: "not-a-number"
```

#### `integer()`
Field must be a valid integer.

```rust
.rule("count", vec![integer()])
// Valid: "42"
// Invalid: "42.5"
```

#### `min(value)`
Numeric field must be at least the given value.

```rust
.rule("age", vec![numeric(), min(18.0)])
```

#### `max(value)`
Numeric field must not exceed the given value.

```rust
.rule("age", vec![numeric(), max(100.0)])
```

---

### Type Validation Rules

#### `string()`
Field must be a string (always passes for form data).

```rust
.rule("name", vec![string()])
```

#### `boolean()`
Field must be a boolean value.

```rust
.rule("accept_terms", vec![boolean()])
// Valid: "true", "false", "1", "0", "yes", "no", "on", "off"
// Invalid: "maybe"
```

#### `array()`
Field is treated as an array (comma-separated values).

```rust
.rule("tags", vec![array()])
```

---

### Format Validation Rules

#### `ip()`
Field must be a valid IP address (IPv4 or IPv6).

```rust
.rule("ip_address", vec![ip()])
// Valid: "192.168.1.1", "2001:0db8:85a3::7334"
// Invalid: "not-an-ip"
```

#### `uuid()`
Field must be a valid UUID.

```rust
.rule("id", vec![uuid()])
// Valid: "550e8400-e29b-41d4-a716-446655440000"
// Invalid: "not-a-uuid"
```

---

### Comparison Rules

#### `confirmed()`
Field must have a matching confirmation field.

```rust
.rule("password", vec![confirmed()])
// Requires: password_confirmation field with same value
```

#### `same(field)`
Field must match another field.

```rust
.rule("email_confirm", vec![same("email")])
```

#### `different(field)`
Field must be different from another field.

```rust
.rule("new_password", vec![different("old_password")])
```

---

### List Validation Rules

#### `in_list(values)`
Field must be one of the given values.

```rust
.rule("status", vec![
    in_list(vec!["active".to_string(), "inactive".to_string(), "pending".to_string()])
])
```

#### `not_in(values)`
Field must not be one of the given values.

```rust
.rule("username", vec![
    not_in(vec!["admin".to_string(), "root".to_string()])
])
```

---

### Date Validation Rules

#### `date()`
Field must be a valid date in YYYY-MM-DD format.

```rust
.rule("birthdate", vec![date()])
// Valid: "1990-05-15"
// Invalid: "15/05/1990", "1990-13-45"
```

#### `before(date)`
Field must be before the given date.

```rust
.rule("start_date", vec![date(), before("2025-12-31")])
```

#### `after(date)`
Field must be after the given date.

```rust
.rule("end_date", vec![date(), after("2020-01-01")])
```

---

## Validator API

### Basic Usage

```rust
use foundry_forms::validation::*;

let data = ValidationData::from(your_hashmap);

let result = Validator::new(data)
    .rule("field1", vec![required(), email()])
    .rule("field2", vec![numeric(), min(0.0)])
    .validate();
```

### Custom Error Messages

```rust
Validator::new(data)
    .rule("email", vec![required(), email()])
    .message("email", "Please provide a valid email address")
    .validate()
```

### Validating Multiple Fields

```rust
Validator::new(data)
    .rule("name", vec![required(), min_length(3)])
    .rule("email", vec![required(), email()])
    .rule("age", vec![required(), integer(), min(18.0), max(100.0)])
    .rule("password", vec![required(), min_length(8), confirmed()])
    .validate()
```

---

## FormRequest Pattern

The FormRequest trait provides Laravel-style form validation with authorization.

### Basic FormRequest

```rust
use foundry_forms::form_request::*;
use foundry_forms::validation::*;
use std::collections::HashMap;

struct CreateUserRequest;

impl FormRequest for CreateUserRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required(), min_length(3)])
            .rule("email", vec![required(), email()])
            .rule("password", vec![required(), min_length(8), confirmed()])
    }

    fn authorize(&self) -> bool {
        // Add authorization logic
        true
    }

    fn messages(&self) -> HashMap<String, String> {
        let mut messages = HashMap::new();
        messages.insert(
            "email".to_string(),
            "Please provide a valid email address".to_string()
        );
        messages
    }
}

// Use it
let request = CreateUserRequest;
let data = ValidationData::from(your_data);

match request.validate(data) {
    Ok(validated_data) => {
        // Process validated data
    }
    Err(FormRequestError::Validation(errors)) => {
        // Handle validation errors
    }
    Err(FormRequestError::Unauthorized) => {
        // Handle unauthorized request
    }
}
```

### FormRequest with Authorization

```rust
struct UpdateProfileRequest {
    user_id: String,
}

impl FormRequest for UpdateProfileRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![required()])
            .rule("bio", vec![max_length(500)])
    }

    fn authorize(&self) -> bool {
        // Check if current user can update this profile
        // Example: current_user.id == self.user_id
        true
    }
}
```

### Conditional Validation in FormRequest

```rust
struct ConditionalRequest;

impl FormRequest for ConditionalRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("role", vec![required(), in_list(vec![
                "user".to_string(),
                "admin".to_string()
            ])])
            .rule("admin_code", vec![required_if("role", "admin")])
    }
}
```

---

## Custom Validation Rules

### Using the `custom()` Function

```rust
use foundry_forms::validation::*;

let custom_rule = custom("no_admin", |_field, value, _data| {
    if let Some(v) = value {
        if v.contains("admin") {
            return Err("Username cannot contain 'admin'".to_string());
        }
    }
    Ok(())
});

Validator::new(data)
    .rule("username", vec![required(), custom_rule])
    .validate()
```

### Using the `custom_rule!` Macro

```rust
use foundry_forms::custom_rule;

let uppercase_rule = custom_rule!("uppercase", |field, value, data| {
    if let Some(v) = value {
        if v != v.to_uppercase() {
            return Err(format!("The {} field must be uppercase", field));
        }
    }
    Ok(())
});

Validator::new(data)
    .rule("code", vec![required(), uppercase_rule])
    .validate()
```

### Creating Reusable Custom Rules

```rust
fn phone_number() -> Box<dyn ValidationRuleTrait> {
    custom("phone", |field, value, _data| {
        if let Some(v) = value {
            if !v.is_empty() {
                let re = regex::Regex::new(r"^\+?[1-9]\d{1,14}$").unwrap();
                if !re.is_match(v) {
                    return Err(format!("The {} field must be a valid phone number", field));
                }
            }
        }
        Ok(())
    })
}

// Use it
Validator::new(data)
    .rule("phone", vec![required(), phone_number()])
    .validate()
```

### Complex Custom Rules with Dependencies

```rust
fn min_age(min_age: u8) -> Box<dyn ValidationRuleTrait> {
    custom("min_age", move |field, value, data| {
        if let Some(birthdate) = value {
            // Parse birthdate and calculate age
            // Return error if under minimum age
            Ok(())
        } else {
            Ok(())
        }
    })
}

Validator::new(data)
    .rule("birthdate", vec![required(), date(), min_age(18)])
    .validate()
```

---

## Error Handling

### ValidationErrors Structure

```rust
pub struct ValidationErrors {
    pub errors: HashMap<String, Vec<String>>,
}
```

### Getting All Errors

```rust
let errors = validator.validate().unwrap_err();

for (field, messages) in errors.errors {
    println!("Field: {}", field);
    for message in messages {
        println!("  - {}", message);
    }
}
```

### Getting Errors for a Specific Field

```rust
if let Some(messages) = errors.get("email") {
    for message in messages {
        println!("{}", message);
    }
}
```

### Getting First Error for a Field

```rust
if let Some(first_error) = errors.first("email") {
    println!("Email error: {}", first_error);
}
```

### Getting All Error Messages

```rust
let all_messages = errors.all();
for message in all_messages {
    println!("{}", message);
}
```

### Checking if There Are Errors

```rust
if errors.has_errors() {
    println!("Validation failed");
}
```

---

## Advanced Usage

### Combining Multiple Rules

```rust
Validator::new(data)
    .rule("email", vec![
        required(),
        email(),
        min_length(5),
        max_length(255)
    ])
    .validate()
```

### Conditional Validation

```rust
let mut validator = Validator::new(data);

// Always validate email
validator = validator.rule("email", vec![required(), email()]);

// Conditionally validate admin code
if is_admin {
    validator = validator.rule("admin_code", vec![required(), size(6)]);
}

validator.validate()
```

### Validating Nested Data

```rust
// For nested fields, use dot notation in field names
Validator::new(data)
    .rule("user.name", vec![required()])
    .rule("user.email", vec![required(), email()])
    .rule("address.city", vec![required()])
    .validate()
```

### Password Confirmation Example

```rust
Validator::new(data)
    .rule("password", vec![
        required(),
        min_length(8),
        regex(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)"), // At least one lowercase, uppercase, and digit
        confirmed()
    ])
    .message("password", "Password must be at least 8 characters with uppercase, lowercase, and numbers")
    .validate()
```

### Multiple Custom Messages

```rust
Validator::new(data)
    .rule("email", vec![required(), email()])
    .rule("password", vec![required(), min_length(8)])
    .message("email", "We need your email to create your account")
    .message("password", "Password must be at least 8 characters long")
    .validate()
```

### Validating Arrays/Lists

```rust
// For comma-separated values
let data = data_with(vec![
    ("tags", "rust,validation,forms")
]);

Validator::new(data)
    .rule("tags", vec![required(), array()])
    .validate()
```

---

## Complete Example: User Registration

```rust
use foundry_forms::form_request::*;
use foundry_forms::validation::*;
use std::collections::HashMap;

struct RegisterUserRequest;

impl FormRequest for RegisterUserRequest {
    fn rules(&self) -> FormRequestValidator {
        FormRequestValidator::new()
            .rule("name", vec![
                required(),
                min_length(3),
                max_length(50),
                alpha()
            ])
            .rule("email", vec![
                required(),
                email(),
                max_length(255)
            ])
            .rule("password", vec![
                required(),
                min_length(8),
                max_length(100),
                regex(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)"),
                confirmed()
            ])
            .rule("age", vec![
                required(),
                integer(),
                min(18.0),
                max(120.0)
            ])
            .rule("terms", vec![
                required(),
                boolean()
            ])
    }

    fn authorize(&self) -> bool {
        true // Public endpoint
    }

    fn messages(&self) -> HashMap<String, String> {
        let mut messages = HashMap::new();
        messages.insert(
            "email".to_string(),
            "Please provide a valid email address".to_string()
        );
        messages.insert(
            "password".to_string(),
            "Password must contain uppercase, lowercase, and numbers".to_string()
        );
        messages.insert(
            "age".to_string(),
            "You must be at least 18 years old to register".to_string()
        );
        messages.insert(
            "terms".to_string(),
            "You must accept the terms and conditions".to_string()
        );
        messages
    }
}

// Usage
fn handle_registration(form_data: HashMap<String, String>) {
    let request = RegisterUserRequest;
    let data = ValidationData::from(form_data);

    match request.validate(data) {
        Ok(validated_data) => {
            // Create user account
            println!("Registration successful!");
        }
        Err(FormRequestError::Validation(errors)) => {
            // Return validation errors to user
            for (field, messages) in errors.errors {
                println!("{}: {:?}", field, messages);
            }
        }
        Err(FormRequestError::Unauthorized) => {
            println!("Unauthorized");
        }
    }
}
```

---

## Rule Count Summary

The validation system includes **27 built-in rules**:

**Required Rules (3):**
- required
- required_if
- required_with

**String Rules (5):**
- email
- url
- alpha
- alpha_numeric
- regex

**Length Rules (4):**
- min_length
- max_length
- between
- size

**Numeric Rules (5):**
- numeric
- integer
- min
- max

**Type Rules (3):**
- string
- boolean
- array

**Format Rules (2):**
- ip
- uuid

**Comparison Rules (3):**
- confirmed
- same
- different

**List Rules (2):**
- in_list
- not_in

**Date Rules (3):**
- date
- before
- after

Plus unlimited custom rules!

---

## Best Practices

1. **Combine Rules Logically**: Put required rules first, then type rules, then constraints
   ```rust
   vec![required(), email(), min_length(5)]
   ```

2. **Use FormRequest for Complex Validation**: Encapsulate validation logic in FormRequest implementations

3. **Provide Clear Error Messages**: Override default messages for better UX

4. **Validate Early**: Validate data at the entry point (controller/handler)

5. **Create Reusable Custom Rules**: For domain-specific validation

6. **Test Your Validation**: Write tests for all validation scenarios

---

## Performance Tips

- Validation rules stop at the first error per field
- Use specific rules before expensive ones (e.g., `required()` before `regex()`)
- Regex patterns are compiled once per validation
- Consider caching compiled regex patterns for frequently used custom rules

---

## Troubleshooting

### Common Issues

**Q: Email validation is passing empty strings**
A: Email rule doesn't enforce required. Use both:
```rust
vec![required(), email()]
```

**Q: How to validate password confirmation?**
A: Use the `confirmed()` rule and ensure the confirmation field exists:
```rust
// data must have: password and password_confirmation
.rule("password", vec![required(), confirmed()])
```

**Q: Custom error message not showing**
A: Make sure to call `.message()` after `.rule()`:
```rust
.rule("email", vec![required()])
.message("email", "Custom message")
```

---

## Next Steps

- Explore the [FormRequest Pattern](#formrequest-pattern) for cleaner code
- Learn about [Custom Validation Rules](#custom-validation-rules)
- Check out the [complete examples](#complete-example-user-registration)
- Read the API documentation for advanced usage
