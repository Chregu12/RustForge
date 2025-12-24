# Phase 21: High-Priority Laravel-Style Macros

This document describes the 4 high-priority macros implemented for RustForge framework Phase 21.

## 1. `routes!` Macro (HIGHEST PRIORITY)

**Purpose**: Solves the German keyboard || problem by providing clean routing syntax without pipes.

**Location**: `/Users/christian/Developer/Github_Projekte/Rust_DX-Framework/crates/rf-macros/src/laravel_macros.rs`

### Basic Usage

```rust
use rf_macros::routes;

routes! {
    get "/posts" => post_controller::index,
    post "/posts" => post_controller::store,
    get "/posts/{id}" => post_controller::show,
    put "/posts/{id}" => post_controller::update,
    delete "/posts/{id}" => post_controller::destroy,
}
```

### With Middleware

```rust
routes! {
    middleware ["auth"] {
        get "/profile" => profile_controller::show,
        put "/profile" => profile_controller::update,
    }

    middleware ["auth", "verified"] {
        get "/dashboard" => dashboard_controller::index,
    }
}
```

### With Prefix

```rust
routes! {
    prefix "/api/v1" {
        get "/users" => api::users::index,
        post "/users" => api::users::store,
    }

    prefix "/admin" {
        middleware ["admin"] {
            get "/stats" => admin::stats::index,
        }
    }
}
```

## 2. `resource!` Macro

**Purpose**: Generate RESTful resource routes automatically.

### Full Resource

```rust
use rf_macros::resource;

// Generates all 7 RESTful routes:
// GET    /posts           - index
// GET    /posts/create    - create
// POST   /posts           - store
// GET    /posts/{id}      - show
// GET    /posts/{id}/edit - edit
// PUT    /posts/{id}      - update
// DELETE /posts/{id}      - destroy
resource!(posts, PostController);
```

### Only Specific Routes

```rust
// Only generate index and show routes
resource!(users, UserController, only: [index, show]);
```

### All Except Specific Routes

```rust
// All routes except destroy
resource!(comments, CommentController, except: [destroy]);
```

## 3. `migration!` Macro

**Purpose**: Define database migrations with Laravel-style syntax.

### Create Table Migration

```rust
use rf_macros::migration;

migration! {
    create_table users {
        id: primary,
        email: string unique,
        name: string,
        password: string,
        role: string = "user",
        timestamps,
    }
}
```

### Column Types Supported

- `primary` - Auto-incrementing primary key
- `string` - VARCHAR column
  - `string unique` - Add unique constraint
  - `string nullable` - Allow NULL values
  - `string = "default"` - Set default value
- `integer` / `i32` / `i64` - Integer column
- `bool` / `boolean` - Boolean column
  - `bool = false` - Set default value
- `timestamps` - Adds created_at and updated_at columns

### Example with Various Column Types

```rust
migration! {
    create_table posts {
        id: primary,
        user_id: integer,
        title: string,
        body: string,
        published: bool = false,
        view_count: integer = 0,
        slug: string unique,
        timestamps,
    }
}
```

## 4. `model!` Macro

**Purpose**: Define models with relationships using Laravel-style syntax.

### Basic Model with Relationships

```rust
use rf_macros::model;

model! {
    Post => "posts" {
        id: i32 primary,
        user_id: i32,
        title: String,
        content: String,
        published: bool = false,
        timestamps,

        belongs_to User via user_id,
        has_many Comment,
    }
}
```

### Relationship Types

- `belongs_to ModelName via foreign_key` - Many-to-one relationship
- `has_many ModelName` - One-to-many relationship

### Generated Code

The macro generates:
- A struct with the specified fields
- `table_name()` method
- Relationship methods (e.g., `user()`, `comments()`)
- `serde` traits for serialization

### Example: Complete Blog Models

```rust
model! {
    User => "users" {
        id: i32 primary,
        name: String,
        email: String,
        timestamps,

        has_many Post,
    }
}

model! {
    Post => "posts" {
        id: i32 primary,
        user_id: i32,
        title: String,
        content: String,
        timestamps,

        belongs_to User via user_id,
        has_many Comment,
    }
}

model! {
    Comment => "comments" {
        id: i32 primary,
        post_id: i32,
        body: String,
        timestamps,

        belongs_to Post via post_id,
    }
}
```

## 5. `request!` Macro

**Purpose**: Define form request validation with Laravel-style syntax.

### Basic Validation

```rust
use rf_macros::request;

request! {
    CreateUser {
        email: email,
        name: length(3, 50),
        password: length(8) + uppercase + number,
        age: range(18, 120) | optional,
    }
}
```

### Validation Rules

- `email` - Must be valid email format
- `length(min)` - Minimum length
- `length(min, max)` - Length between min and max
- `range(min, max)` - Numeric range
- `uppercase` - Must contain uppercase letter
- `number` - Must contain a number
- `optional` - Field is optional (uses `Option<String>`)

### Combining Rules

```rust
request! {
    RegisterUser {
        // Multiple rules with +
        password: length(8) + uppercase + number,

        // Optional field with |
        phone: length(10) | optional,

        // Single rule
        email: email,
    }
}
```

### Using in Controllers

```rust
// The generated struct has a validate() method
let req = CreateUser {
    email: "user@example.com".to_string(),
    name: "John Doe".to_string(),
    password: "Secret123".to_string(),
    age: Some("25".to_string()),
};

match req.validate() {
    Ok(_) => {
        // Validation passed
    }
    Err(errors) => {
        // Handle validation errors
        for error in errors {
            println!("Validation error: {}", error);
        }
    }
}
```

## Implementation Status

All 4 macros are implemented and compile successfully:
- ✅ `routes!` - Complete with middleware and prefix support
- ✅ `resource!` - Complete with only/except filters
- ✅ `migration!` - Complete with create_table support
- ✅ `model!` - Complete with belongs_to and has_many relationships
- ✅ `request!` - Complete with validation rules and optional fields

## Files Modified

1. `/Users/christian/Developer/Github_Projekte/Rust_DX-Framework/crates/rf-macros/src/laravel_macros.rs` - New file with all macro implementations
2. `/Users/christian/Developer/Github_Projekte/Rust_DX-Framework/crates/rf-macros/src/lib.rs` - Added module import and macro exports

## Compilation

```bash
cargo check -p rf-macros
```

Status: ✅ Compiles successfully with 23 warnings (all non-critical)

## Next Steps

1. Add comprehensive tests for each macro
2. Add support for more migration operations (add_column, add_index, drop_column)
3. Extend model relationships (has_one, belongs_to_many)
4. Add more validation rules to request! macro
5. Update main framework documentation with these new macros
