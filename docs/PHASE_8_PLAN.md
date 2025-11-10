# Phase 8: Developer Experience & Testing

**Status**: 🚀 Starting
**Date**: 2025-11-10
**Focus**: Testing Utilities, API Documentation, Pagination, File Uploads

## Overview

Phase 8 focuses on developer experience and testing capabilities to make RustForge more productive and enjoyable to work with. This phase adds essential testing tools, automatic API documentation, pagination helpers, and file upload utilities.

## Goals

1. **Testing Utilities**: Factories, seeders, HTTP testing, assertions
2. **API Documentation**: OpenAPI/Swagger automatic generation
3. **Pagination**: Cursor and offset pagination with metadata
4. **File Uploads**: Multipart handling, validation, processing

## Priority Features

### 🔴 High Priority

#### 1. Testing Utilities (rf-testing extension)
**Estimated**: 4-5 hours
**Why**: Essential for developer productivity and test quality

**Features**:
- Model factories (builder pattern)
- Database seeding
- HTTP testing helpers
- Response assertions
- Mock services
- Test database transactions

**API Design**:
```rust
use rf_testing::*;

// Factory pattern
let user = UserFactory::new()
    .with_email("test@example.com")
    .with_role("admin")
    .create()
    .await?;

// Seeding
let seeder = DatabaseSeeder::new();
seeder.seed::<UserSeeder>().await?;
seeder.seed::<PostSeeder>().await?;

// HTTP testing
let response = test_client()
    .get("/api/users")
    .header("Authorization", "Bearer token")
    .send()
    .await?;

response.assert_status(200);
response.assert_json_contains(json!({ "id": 1 }));

// Test transactions
test_transaction(|db| async move {
    let user = create_user(&db).await?;
    assert!(user.id > 0);
    // Automatically rolled back
}).await?;
```

**Laravel Parity**: ~85% (Factories, Seeders, HTTP Tests)

#### 2. API Documentation (rf-swagger)
**Estimated**: 5-6 hours
**Why**: Critical for API discoverability and client integration

**Features**:
- OpenAPI 3.0 schema generation
- Automatic endpoint discovery
- Request/response schema extraction
- Swagger UI integration
- ReDoc integration
- Schema validation
- Example generation

**API Design**:
```rust
use rf_swagger::*;

// Annotate endpoints
#[openapi(
    summary = "Get user by ID",
    description = "Returns a user with the given ID",
    tags = ["users"],
)]
async fn get_user(
    Path(id): Path<i32>,
) -> Result<Json<User>, ApiError> {
    // Implementation
}

// Generate OpenAPI spec
let openapi = OpenApiBuilder::new()
    .title("My API")
    .version("1.0.0")
    .description("API for my application")
    .build()?;

// Mount Swagger UI
app.merge(openapi.swagger_ui("/swagger"));
app.merge(openapi.redoc("/redoc"));

// JSON spec endpoint
app.route("/openapi.json", get(openapi.spec_handler()));
```

**Laravel Parity**: ~70% (L5-Swagger)

### 🟡 Medium Priority

#### 3. Pagination (rf-pagination)
**Estimated**: 3-4 hours
**Why**: Almost every API needs pagination

**Features**:
- Offset pagination (page-based)
- Cursor pagination (for large datasets)
- Metadata (total, per_page, current_page)
- Links (first, last, next, prev)
- Configurable defaults
- Query builder integration

**API Design**:
```rust
use rf_pagination::*;

// Offset pagination
let users = User::query()
    .paginate(25)  // 25 per page
    .page(2)       // Page 2
    .execute()
    .await?;

let response = PaginatedResponse {
    data: users.data,
    meta: users.meta,
    links: users.links,
};

// Cursor pagination (more efficient for large datasets)
let posts = Post::query()
    .cursor_paginate(50)
    .after("cursor_value")
    .execute()
    .await?;

// Manual pagination
let paginator = Paginator::new(total_count, per_page, current_page);
let offset = paginator.offset();
let limit = paginator.limit();
```

**Laravel Parity**: ~90% (Pagination)

#### 4. File Upload Helpers (rf-upload)
**Estimated**: 3-4 hours
**Why**: File uploads are common but complex

**Features**:
- Multipart form data parsing
- File type validation (MIME types)
- File size limits
- Image processing (resize, crop)
- Temporary file handling
- Storage integration
- Progress tracking

**API Design**:
```rust
use rf_upload::*;

// Handle file upload
async fn upload_avatar(
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, ApiError> {
    let upload = FileUpload::from_multipart(&mut multipart)
        .await?
        .validate_mime_types(&["image/jpeg", "image/png"])?
        .validate_max_size(5 * 1024 * 1024)?  // 5MB
        .resize(200, 200)?
        .store("avatars")
        .await?;

    Ok(Json(UploadResponse {
        path: upload.path,
        url: upload.url,
        size: upload.size,
    }))
}

// Image processing
let image = Image::open(path)?
    .resize(800, 600, ResizeMode::Fit)?
    .crop(100, 100, 400, 400)?
    .format(ImageFormat::Jpeg)?
    .quality(85)?
    .save("output.jpg")?;

// Multiple files
let files = FileUpload::from_multipart_multiple(&mut multipart).await?;
for file in files {
    file.store("uploads").await?;
}
```

**Laravel Parity**: ~75% (File Uploads + Image intervention)

## Implementation Plan

### Step 1: Testing Utilities
1. Extend `crates/rf-testing/`
2. Implement Factory trait and macros
3. Implement Seeder trait
4. Add HTTP test client
5. Add response assertions
6. Add test transaction support
7. Write tests (8-10 tests)
8. Write documentation

### Step 2: API Documentation
1. Create `crates/rf-swagger/`
2. Implement OpenAPI schema builder
3. Add derive macro for OpenAPI
4. Implement Swagger UI integration
5. Implement ReDoc integration
6. Add schema extraction from types
7. Write tests (6-8 tests)
8. Write documentation

### Step 3: Pagination
1. Create `crates/rf-pagination/`
2. Implement Paginator struct
3. Implement offset pagination
4. Implement cursor pagination
5. Add metadata and links
6. Integrate with query builder
7. Write tests (8-10 tests)
8. Write documentation

### Step 4: File Uploads
1. Create `crates/rf-upload/`
2. Implement FileUpload extractor
3. Add MIME type validation
4. Add size validation
5. Implement image processing
6. Add storage integration
7. Write tests (8-10 tests)
8. Write documentation

## Technical Architecture

### Testing Architecture
```
┌─────────────────┐
│  Factory Trait  │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
┌───▼──┐  ┌──▼───┐
│Model │  │Faker │
│Builder│ │Data  │
└──────┘  └──────┘
```

### OpenAPI Architecture
```
┌─────────────────┐
│ OpenAPI Builder │
│                 │
│  - Routes       │
│  - Schemas      │
│  - Parameters   │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
┌───▼──┐  ┌──▼───┐
│Swagger│ │ReDoc │
│  UI   │ │ UI   │
└──────┘  └──────┘
```

### Pagination Architecture
```
┌─────────────────┐
│   Paginator     │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
┌───▼──┐  ┌──▼───┐
│Offset│  │Cursor│
│ Page │  │ Page │
└──────┘  └──────┘
```

## Dependencies

```toml
# Testing
fake = "2.9"  # Fake data generation
mockall = "0.12"  # Mocking

# OpenAPI
utoipa = "4.2"  # OpenAPI generation
utoipa-swagger-ui = "6.0"  # Swagger UI
utoipa-redoc = "3.0"  # ReDoc

# File Upload
image = "0.25"  # Image processing
mime_guess = "2.0"  # MIME type detection
tempfile = "3.10"  # Temporary files
```

## Success Criteria

### Testing Utilities
- ✅ Factory pattern works for models
- ✅ Database seeding works
- ✅ HTTP test client works
- ✅ Assertions work correctly
- ✅ Test transactions roll back
- ✅ All tests passing

### API Documentation
- ✅ OpenAPI schema generated
- ✅ Swagger UI accessible
- ✅ ReDoc accessible
- ✅ Schemas extracted correctly
- ✅ All tests passing

### Pagination
- ✅ Offset pagination works
- ✅ Cursor pagination works
- ✅ Metadata accurate
- ✅ Links generated correctly
- ✅ All tests passing

### File Uploads
- ✅ Multipart parsing works
- ✅ Validation works
- ✅ Image processing works
- ✅ Storage integration works
- ✅ All tests passing

## Laravel Feature Parity

After Phase 8:
- **Testing**: ~85% (Factories, Seeders, HTTP Tests)
- **API Docs**: ~70% (L5-Swagger equivalent)
- **Pagination**: ~90%
- **File Uploads**: ~75%
- **Overall**: ~96%+ complete framework

---

**Phase 8: Developer experience and testing excellence! 🧪**
