# API Sketch: rf-validation - Validation & Forms

**Component**: rf-validation
**Version**: 0.1.0
**Status**: Draft
**Date**: 2025-01-09

## Overview

Production-ready validation and form handling providing:
- Declarative validation rules with proc macros
- Laravel-inspired validation patterns
- Custom validation rules
- Field-level error messages
- Integration with Axum and Serde
- Type-safe validation
- Internationalization support (i18n-ready)

## Goals

1. **Declarative Validation**: Validate structs with derive macros
2. **Rich Rule Set**: 30+ built-in validation rules (Laravel parity)
3. **Custom Rules**: Easy custom validation logic
4. **Field-Level Errors**: Detailed error messages per field
5. **Type Safety**: Compile-time validation rule checking
6. **Axum Integration**: Automatic validation in handlers
7. **I18n Ready**: Translatable error messages
8. **Performance**: Zero-cost abstractions where possible

## Architecture

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│  (Request Handlers, DTOs)               │
└─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│       rf-validation (Facade)            │
│  • Validate Trait                       │
│  • ValidationRules                      │
│  • ValidationErrors                     │
│  • FormRequest                          │
└─────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│   validator  │ │    regex     │ │     serde    │
│  (derive)    │ │  (patterns)  │ │(serialization)│
└──────────────┘ └──────────────┘ └──────────────┘
```

## Core Components

### 1. Validate Trait

Basic validation trait for any struct.

```rust
use rf_validation::{Validate, ValidationError};

#[derive(Debug, Validate)]
struct RegisterRequest {
    #[validate(email)]
    email: String,

    #[validate(length(min = 8, max = 128))]
    password: String,

    #[validate(length(min = 2, max = 100))]
    name: String,
}

// Usage
let request = RegisterRequest {
    email: "user@example.com".to_string(),
    password: "short".to_string(),
    name: "John".to_string(),
};

match request.validate() {
    Ok(_) => println!("Valid!"),
    Err(errors) => {
        for (field, messages) in errors.field_errors() {
            println!("{}: {:?}", field, messages);
        }
    }
}
```

### 2. Built-in Validation Rules

30+ validation rules covering common use cases:

#### String Validation
```rust
#[derive(Validate)]
struct StringRules {
    #[validate(email)]
    email: String,

    #[validate(url)]
    website: String,

    #[validate(length(min = 3, max = 20))]
    username: String,

    #[validate(regex(pattern = r"^[a-zA-Z0-9_]+$"))]
    slug: String,

    #[validate(contains = "rust")]
    bio: String,

    #[validate(custom = "is_profane")]
    comment: String,
}

fn is_profane(value: &str) -> Result<(), ValidationError> {
    if value.contains("badword") {
        return Err(ValidationError::new("contains profanity"));
    }
    Ok(())
}
```

#### Numeric Validation
```rust
#[derive(Validate)]
struct NumericRules {
    #[validate(range(min = 18, max = 120))]
    age: u32,

    #[validate(range(min = 0.0, max = 100.0))]
    percentage: f64,

    #[validate(custom = "is_even")]
    even_number: i32,
}

fn is_even(value: &i32) -> Result<(), ValidationError> {
    if value % 2 != 0 {
        return Err(ValidationError::new("must be even"));
    }
    Ok(())
}
```

#### Collection Validation
```rust
#[derive(Validate)]
struct CollectionRules {
    #[validate(length(min = 1, max = 5))]
    tags: Vec<String>,

    #[validate]
    nested: Vec<NestedStruct>,
}

#[derive(Validate)]
struct NestedStruct {
    #[validate(length(min = 1))]
    value: String,
}
```

#### Date/Time Validation
```rust
use chrono::{DateTime, Utc};

#[derive(Validate)]
struct DateRules {
    #[validate(custom = "is_future")]
    expires_at: DateTime<Utc>,

    #[validate(custom = "is_past")]
    birth_date: DateTime<Utc>,
}

fn is_future(date: &DateTime<Utc>) -> Result<(), ValidationError> {
    if date <= &Utc::now() {
        return Err(ValidationError::new("must be in the future"));
    }
    Ok(())
}
```

### 3. FormRequest Pattern

Laravel-inspired FormRequest for automatic validation in Axum handlers.

```rust
use rf_validation::{FormRequest, Validate};
use axum::{Json, extract::FromRequest};

#[derive(Debug, Validate, FormRequest)]
struct CreateUserRequest {
    #[validate(email)]
    email: String,

    #[validate(length(min = 8))]
    password: String,

    #[validate(length(min = 2, max = 100))]
    name: String,

    #[validate(range(min = 18, max = 120))]
    age: u32,
}

// Automatic validation in handler
async fn create_user(
    form: CreateUserRequest,  // Automatically validated!
) -> Json<UserResponse> {
    // If we get here, validation passed
    let user = User::create(
        form.email,
        form.password,
        form.name,
        form.age,
    ).await?;

    Json(UserResponse::from(user))
}
```

### 4. Validation Errors

Detailed error information per field.

```rust
use rf_validation::ValidationErrors;

let errors = request.validate().unwrap_err();

// Get all field errors
for (field, field_errors) in errors.field_errors() {
    println!("Field '{}' has errors:", field);
    for error in field_errors {
        println!("  - {}", error.message);
        println!("    Code: {}", error.code);
        if let Some(params) = &error.params {
            println!("    Params: {:?}", params);
        }
    }
}

// Get errors for specific field
if let Some(email_errors) = errors.field_errors().get("email") {
    println!("Email errors: {:?}", email_errors);
}

// Convert to JSON for API responses
let json = serde_json::to_string_pretty(&errors)?;
println!("{}", json);
```

### 5. Custom Validation Rules

Create reusable custom validators.

```rust
use rf_validation::{Validator, ValidationError, ValidationResult};

struct UniqueEmailValidator {
    db: Arc<DatabaseManager>,
}

impl Validator<String> for UniqueEmailValidator {
    fn validate(&self, value: &String) -> ValidationResult {
        // Check if email exists in database
        let exists = self.db.user_exists_by_email(value).await?;

        if exists {
            return Err(ValidationError::new("email already exists"));
        }

        Ok(())
    }

    fn code(&self) -> &str {
        "unique_email"
    }
}

// Usage in validation
#[derive(Validate)]
struct RegisterRequest {
    #[validate(custom(function = "validate_unique_email"))]
    email: String,
}

fn validate_unique_email(email: &str) -> ValidationResult {
    let validator = UniqueEmailValidator { db: get_db() };
    validator.validate(&email.to_string())
}
```

### 6. Conditional Validation

Validate fields based on other field values.

```rust
#[derive(Validate)]
struct PaymentRequest {
    #[validate(length(min = 1))]
    payment_method: String,

    // Only validate if payment_method == "credit_card"
    #[validate(required_if(field = "payment_method", value = "credit_card"))]
    #[validate(credit_card)]
    card_number: Option<String>,

    // Only validate if payment_method == "paypal"
    #[validate(required_if(field = "payment_method", value = "paypal"))]
    #[validate(email)]
    paypal_email: Option<String>,
}
```

### 7. Complete Rule Reference

```rust
// String rules
#[validate(email)]                          // Valid email address
#[validate(url)]                            // Valid URL
#[validate(length(min = 3, max = 20))]      // Length constraints
#[validate(regex(pattern = r"^\d+$"))]      // Regex pattern
#[validate(contains = "substring")]          // Contains substring
#[validate(starts_with = "prefix")]         // Starts with prefix
#[validate(ends_with = "suffix")]           // Ends with suffix
#[validate(alpha)]                          // Only alphabetic
#[validate(alphanumeric)]                   // Alphanumeric only
#[validate(lowercase)]                      // Lowercase only
#[validate(uppercase)]                      // Uppercase only
#[validate(credit_card)]                    // Valid credit card (Luhn algorithm)

// Numeric rules
#[validate(range(min = 0, max = 100))]      // Value range
#[validate(positive)]                       // Positive number
#[validate(negative)]                       // Negative number

// Collection rules
#[validate(length(min = 1, max = 10))]      // Collection size
#[validate(unique)]                         // Unique elements
#[validate(contains(item = "value"))]       // Contains specific item

// Boolean rules
#[validate(must_be_true)]                   // Must be true
#[validate(accepted)]                       // Accepted (true, "1", "yes", "on")

// Comparison rules
#[validate(equal_to(field = "password"))]   // Equal to another field
#[validate(not_equal_to(field = "old_password"))] // Not equal to field

// Conditional rules
#[validate(required)]                       // Required field
#[validate(required_if(field = "x", value = "y"))] // Required if condition
#[validate(required_unless(field = "x", value = "y"))] // Required unless
#[validate(required_with(field = "x"))]     // Required if x is present
#[validate(required_without(field = "x"))]  // Required if x is absent

// Custom rules
#[validate(custom = "my_validator")]        // Custom function
#[validate(custom(function = "async_validator"))] // Async custom validator
```

### 8. Nested Validation

Validate nested structures and collections.

```rust
#[derive(Validate)]
struct CreatePostRequest {
    #[validate(length(min = 5, max = 200))]
    title: String,

    #[validate(length(min = 10))]
    content: String,

    // Validate nested struct
    #[validate]
    author: Author,

    // Validate each tag in collection
    #[validate(length(min = 1, max = 5))]
    tags: Vec<String>,

    // Validate nested structs in collection
    #[validate]
    comments: Vec<Comment>,
}

#[derive(Validate)]
struct Author {
    #[validate(email)]
    email: String,

    #[validate(length(min = 2))]
    name: String,
}

#[derive(Validate)]
struct Comment {
    #[validate(length(min = 1, max = 500))]
    text: String,
}
```

### 9. Error Messages

Customizable error messages with i18n support.

```rust
#[derive(Validate)]
struct LoginRequest {
    #[validate(email, message = "Please provide a valid email address")]
    email: String,

    #[validate(
        length(min = 8),
        message = "Password must be at least 8 characters long"
    )]
    password: String,
}

// Or use message keys for i18n
#[derive(Validate)]
struct RegisterRequest {
    #[validate(email, message_key = "validation.email")]
    email: String,
}

// Message resolution
let messages = HashMap::from([
    ("validation.email", "The email field must be a valid email address."),
    ("validation.length.min", "The {field} must be at least {min} characters."),
]);
```

### 10. Axum Integration

Complete integration with Axum request handling.

```rust
use rf_validation::{FormRequest, Validate, ValidatedJson};
use axum::{Json, extract::FromRequest};

// Automatic validation with FormRequest
async fn create_user(
    form: CreateUserRequest,  // Automatically validated
) -> Json<UserResponse> {
    // Validation passed, safe to use
    create_user_in_db(form).await
}

// Manual validation with ValidatedJson
async fn update_user(
    ValidatedJson(form): ValidatedJson<UpdateUserRequest>,
) -> Json<UserResponse> {
    // Validation passed
    update_user_in_db(form).await
}

// Custom validation in handler
async fn custom_validation(
    Json(mut form): Json<MyRequest>,
) -> Result<Json<Response>, AppError> {
    // Validate
    form.validate()?;

    // Additional custom logic
    if form.email.contains("spam") {
        return Err(AppError::BadRequest {
            message: "Email not allowed".to_string(),
        });
    }

    Ok(Json(process(form)))
}
```

### 11. Validation Groups

Validate different rules based on context.

```rust
#[derive(Validate)]
struct User {
    #[validate(email, groups = ["create", "update"])]
    email: String,

    #[validate(length(min = 8), groups = ["create"])]
    password: Option<String>,

    #[validate(length(min = 2), groups = ["create", "update"])]
    name: String,
}

// Validate only "create" group
user.validate_group("create")?;

// Validate only "update" group
user.validate_group("update")?;
```

### 12. Async Validation

Support for async validators (database lookups, API calls).

```rust
use rf_validation::{AsyncValidator, ValidationResult};

struct UniqueEmailValidator {
    db: Arc<DatabaseManager>,
}

#[async_trait]
impl AsyncValidator<String> for UniqueEmailValidator {
    async fn validate(&self, value: &String) -> ValidationResult {
        let exists = User::find()
            .filter(user::Column::Email.eq(value))
            .one(self.db.connection())
            .await?
            .is_some();

        if exists {
            return Err(ValidationError::new("email already exists"));
        }

        Ok(())
    }
}

// Usage
#[derive(Validate)]
struct RegisterRequest {
    #[validate(async_custom = "validate_unique_email")]
    email: String,
}

async fn validate_unique_email(email: &str) -> ValidationResult {
    let validator = UniqueEmailValidator { db: get_db() };
    validator.validate(&email.to_string()).await
}
```

## Configuration

### Validation Config

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Default locale for error messages
    #[serde(default = "default_locale")]
    pub locale: String,

    /// Stop validation on first error
    #[serde(default)]
    pub bail_on_first_error: bool,

    /// Custom error message templates
    #[serde(default)]
    pub messages: HashMap<String, String>,
}

fn default_locale() -> String {
    "en".to_string()
}
```

### TOML Configuration

```toml
[validation]
locale = "en"
bail_on_first_error = false

[validation.messages]
"email" = "The {field} must be a valid email address."
"length.min" = "The {field} must be at least {min} characters."
"length.max" = "The {field} may not be greater than {max} characters."
```

## Error Response Format

```json
{
  "type": "validation-failed",
  "title": "Validation Failed",
  "status": 422,
  "detail": "One or more fields failed validation",
  "errors": {
    "email": [
      {
        "code": "email",
        "message": "The email field must be a valid email address.",
        "params": {
          "value": "invalid-email"
        }
      }
    ],
    "password": [
      {
        "code": "length",
        "message": "The password must be at least 8 characters.",
        "params": {
          "min": 8,
          "actual": 5
        }
      }
    ]
  }
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        #[derive(Validate)]
        struct Form {
            #[validate(email)]
            email: String,
        }

        let valid = Form {
            email: "user@example.com".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = Form {
            email: "not-an-email".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_length_validation() {
        #[derive(Validate)]
        struct Form {
            #[validate(length(min = 3, max = 10))]
            username: String,
        }

        let valid = Form {
            username: "john".to_string(),
        };
        assert!(valid.validate().is_ok());

        let too_short = Form {
            username: "ab".to_string(),
        };
        assert!(too_short.validate().is_err());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_form_request_validation() {
    #[derive(Validate, FormRequest)]
    struct CreateUserRequest {
        #[validate(email)]
        email: String,
    }

    let app = Router::new()
        .route("/users", post(create_user));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/users")
                .method("POST")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"email": "invalid"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
```

## Performance

- **Compile-time**: Proc macros generate validation code at compile time
- **Zero-cost**: No runtime overhead for basic validations
- **Lazy**: Validations only run when called
- **Parallel**: Independent field validations can run concurrently
- **Cached**: Regex patterns compiled once and cached

## Summary

rf-validation provides:
- ✅ 30+ built-in validation rules
- ✅ Declarative validation with derive macros
- ✅ Custom validation rules (sync + async)
- ✅ FormRequest pattern for Axum
- ✅ Field-level error messages
- ✅ Nested validation support
- ✅ Conditional validation
- ✅ I18n-ready error messages
- ✅ RFC 7807 error responses
- ✅ Type-safe validation

Next: Implementation in `crates/rf-validation/`
