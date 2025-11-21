# API Versioning Guide

Complete guide to implementing API versioning in RustForge applications.

## Overview

RustForge provides flexible API versioning support with three different strategies:

1. **URL-based versioning**: `/v1/users`, `/v2/users`
2. **Header-based versioning**: `Accept: application/vnd.api.v1+json`
3. **Custom header versioning**: `API-Version: 1`

## Quick Start

### URL-Based Versioning

The simplest approach - embed version in URL path:

```rust
use rf_routing::versioned_router::VersionedRouterBuilder;
use axum::{routing::get, Router};

let app = VersionedRouterBuilder::new()
    .version(1, |router| {
        router.route("/users", get(get_users_v1))
    })
    .version(2, |router| {
        router.route("/users", get(get_users_v2))
    })
    .default_version(2)
    .build_with_prefix();  // Creates /v1/users and /v2/users
```

### Header-Based Versioning

Use Accept header for content negotiation:

```rust
use rf_routing::versioning::ApiVersion;

async fn handler(version: ApiVersion) -> String {
    match version.version() {
        1 => "Version 1 response".to_string(),
        2 => "Version 2 response".to_string(),
        _ => "Latest version".to_string(),
    }
}
```

Client request:

```bash
curl -H "Accept: application/vnd.api.v1+json" https://api.example.com/users
```

### Custom Header Versioning

Simple numeric header:

```bash
curl -H "API-Version: 2" https://api.example.com/users
```

## Version Configuration

### Setting Up Versions

```rust
use rf_routing::{
    versioned_router::VersionedRouterBuilder,
    versioning::VersionConfig,
};

let app = VersionedRouterBuilder::new()
    .version(1, configure_v1)
    .version(2, configure_v2)
    .version(3, configure_v3)
    .default_version(3)
    .supported_versions(vec![1, 2, 3])
    .deprecated_versions(vec![])  // Mark old versions as deprecated
    .build();
```

### Version Negotiation

Control how versions are selected:

```rust
use rf_routing::versioning::{DefaultNegotiator, VersionNegotiator, VersionConfig};

let config = VersionConfig {
    default_version: 2,
    supported_versions: vec![1, 2, 3],
    deprecated_versions: vec![],
};

let negotiator = DefaultNegotiator::new(config);

// Negotiate version
match negotiator.negotiate(Some(1)) {
    Ok(version) => println!("Using version {}", version),
    Err(e) => println!("Error: {}", e),
}
```

## Practical Examples

### Example 1: Evolving Data Models

```rust
// Version 1: Basic user
#[derive(Serialize)]
struct UserV1 {
    id: i64,
    name: String,
}

// Version 2: Added email
#[derive(Serialize)]
struct UserV2 {
    id: i64,
    name: String,
    email: String,
}

// Version 3: Added timestamps
#[derive(Serialize)]
struct UserV3 {
    id: i64,
    name: String,
    email: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Handlers for each version
async fn get_user_v1(Path(id): Path<i64>) -> Json<UserV1> {
    // Return V1 format
}

async fn get_user_v2(Path(id): Path<i64>) -> Json<UserV2> {
    // Return V2 format
}

async fn get_user_v3(Path(id): Path<i64>) -> Json<UserV3> {
    // Return V3 format
}
```

### Example 2: Breaking Changes

When you need to make breaking changes:

```rust
// V1: Returns array directly
async fn get_products_v1() -> Json<Vec<Product>> {
    Json(products)
}

// V2: Wrapped in envelope with metadata
async fn get_products_v2() -> Json<ProductResponse> {
    Json(ProductResponse {
        data: products,
        meta: Metadata {
            total: 100,
            page: 1,
        },
    })
}
```

### Example 3: Different Authentication

Different versions can use different auth:

```rust
use rf_routing::versioned_router::VersionedRouterBuilder;

let app = VersionedRouterBuilder::new()
    .version(1, |router| {
        router
            .route("/users", get(get_users))
            .layer(basic_auth_middleware())  // V1 uses basic auth
    })
    .version(2, |router| {
        router
            .route("/users", get(get_users))
            .layer(oauth2_middleware())  // V2 uses OAuth2
    })
    .build_with_prefix();
```

## Best Practices

### 1. Semantic Versioning

Use simple integers, not semantic versions:

- ✅ Good: `/v1/`, `/v2/`, `/v3/`
- ❌ Bad: `/v1.2.3/`, `/v2.0.0-beta/`

### 2. Default to Latest

Always default to the latest stable version:

```rust
.default_version(3)  // Latest stable
```

### 3. Sunset Old Versions

Deprecate gradually:

```rust
.deprecated_versions(vec![1])  // V1 is deprecated
```

Return deprecation warnings:

```rust
async fn handler(version: ApiVersion) -> Response {
    if version.version() == 1 {
        // Add deprecation header
        let mut response = /* ... */;
        response.headers_mut().insert(
            "Sunset",
            "Sat, 31 Dec 2024 23:59:59 GMT".parse().unwrap()
        );
        response
    }
}
```

### 4. Document Version Differences

Maintain a changelog for each version:

```markdown
## Version 3 (Current)
- Added `created_at` and `updated_at` to User
- Changed response format to envelope style

## Version 2 (Supported)
- Added `email` to User
- Deprecated `/auth/token` endpoint

## Version 1 (Deprecated - Sunset: 2024-12-31)
- Original API
```

### 5. Test All Versions

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_v1_endpoint() {
        let app = create_app();
        let response = app
            .oneshot(Request::get("/v1/users").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_v2_endpoint() {
        // Test V2
    }
}
```

## Version Extraction Methods

### From URL Path

```rust
use rf_routing::versioning::extract_from_path;

let version = extract_from_path("/v2/users");
assert_eq!(version, Some(2));
```

### From Accept Header

```rust
use rf_routing::versioning::extract_from_accept;

let version = extract_from_accept("application/vnd.api.v2+json");
assert_eq!(version, Some(2));
```

### From Custom Header

```rust
use rf_routing::versioning::extract_from_header;

let version = extract_from_header("2");
assert_eq!(version, Some(2));
```

## Error Handling

Handle version errors gracefully:

```rust
use rf_routing::versioning::{ApiVersion, VersionError};

async fn handler(
    version: Result<ApiVersion, VersionError>
) -> Response {
    match version {
        Ok(v) => {
            // Handle request with version v
        },
        Err(VersionError::MissingVersion) => {
            // Use default version
        },
        Err(VersionError::UnsupportedVersion(v)) => {
            (
                StatusCode::NOT_ACCEPTABLE,
                format!("Version {} is not supported", v)
            ).into_response()
        },
        Err(e) => {
            (StatusCode::BAD_REQUEST, e.to_string()).into_response()
        }
    }
}
```

## Migration Strategy

### Phase 1: Introduce Versioning

1. Current API becomes V1
2. Add versioning infrastructure
3. Test V1 with version prefix

### Phase 2: Develop V2

1. Create V2 routes alongside V1
2. Both versions active
3. Encourage migration to V2

### Phase 3: Deprecate V1

1. Mark V1 as deprecated
2. Add sunset date header
3. Send deprecation warnings to V1 users

### Phase 4: Remove V1

1. Remove V1 routes
2. V2 becomes the baseline
3. Repeat for V3, V4, etc.

## Common Patterns

### Pattern 1: Shared Business Logic

```rust
// Shared service layer
async fn get_user_data(id: i64, db: &Database) -> User {
    // Business logic
}

// Different response transformers
async fn get_user_v1(id: Path<i64>) -> Json<UserV1> {
    let user = get_user_data(*id, &db).await;
    Json(user.into_v1())
}

async fn get_user_v2(id: Path<i64>) -> Json<UserV2> {
    let user = get_user_data(*id, &db).await;
    Json(user.into_v2())
}
```

### Pattern 2: Version-Aware Resources

```rust
use rf_api_resources::ResourceBuilder;

fn user_resource(user: &User, version: u32) -> Value {
    let mut resource = ResourceBuilder::new()
        .add("id", user.id)
        .add("name", &user.name);

    resource = resource.when(version >= 2, |r| {
        r.add("email", &user.email)
    });

    resource = resource.when(version >= 3, |r| {
        r.add("created_at", user.created_at)
         .add("updated_at", user.updated_at)
    });

    resource.build()
}
```

### Pattern 3: Feature Flags per Version

```rust
struct VersionFeatures {
    has_pagination: bool,
    has_filtering: bool,
    max_page_size: usize,
}

fn features_for_version(v: u32) -> VersionFeatures {
    match v {
        1 => VersionFeatures {
            has_pagination: false,
            has_filtering: false,
            max_page_size: 100,
        },
        2 => VersionFeatures {
            has_pagination: true,
            has_filtering: false,
            max_page_size: 100,
        },
        _ => VersionFeatures {
            has_pagination: true,
            has_filtering: true,
            max_page_size: 1000,
        },
    }
}
```

## Monitoring

Track version usage:

```rust
use tracing::info;

async fn handler(version: ApiVersion) -> Response {
    info!(
        version = version.version(),
        "API request"
    );

    // Metrics
    metrics::counter!("api.requests")
        .increment(1);
    metrics::counter!("api.version.{}", version.version())
        .increment(1);

    // Handle request
}
```

## Complete Example

See `examples/versioning_example.rs` for a complete working example.

## Further Reading

- [Semantic Versioning](https://semver.org/)
- [API Evolution Patterns](https://martinfowler.com/articles/enterpriseREST.html)
- [RFC 7231 - Content Negotiation](https://tools.ietf.org/html/rfc7231#section-5.3)
