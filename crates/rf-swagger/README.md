# rf-swagger

OpenAPI / Swagger UI / ReDoc integration for RustForge, built as a thin layer over
[utoipa](https://crates.io/crates/utoipa) 4.x.

## What this crate IS

- A convenience re-export of `utoipa` and `utoipa::ToSchema`
- `OpenApiBuilder` — a metadata builder (`title`, `version`, `description`, `contact`, `license`)
  that produces a `utoipa::openapi::OpenApi` via `build()`
- `swagger_ui(openapi)` — wraps the passed spec in a `SwaggerUi` router (Swagger UI + JSON endpoint)
- `redoc(openapi)` — wraps the passed spec in a `Redoc` router

## What this crate is NOT

- **Not** an automatic route-introspecting spec generator. There is no magic scan of your
  handler functions.
- **Not** an alternative to annotating your handlers with `#[utoipa::path]` and deriving
  `#[derive(utoipa::OpenApi)]`.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
rf-swagger = { path = "../rf-swagger" }
utoipa = { version = "4", features = ["axum_extras"] }
axum = "0.7"   # see compatibility note below
```

Annotate your handlers, derive the spec, then pass it to rf-swagger:

```rust
use rf_swagger::{swagger_ui, OpenApiBuilder};
use utoipa::OpenApi as UtoipaOpenApi;

// 1. Annotate handlers
#[utoipa::path(
    get,
    path = "/users",
    responses((status = 200, description = "list of users"))
)]
async fn list_users() -> &'static str { "[]" }

// 2. Collect into an OpenApi struct
#[derive(utoipa::OpenApi)]
#[openapi(paths(list_users))]
struct ApiDoc;

// 3. Build spec and pass to rf-swagger
let spec = ApiDoc::openapi();
let swagger = rf_swagger::swagger_ui(spec);

// 4. Merge into your axum 0.7 router
// let app = axum::Router::new()
//     .route("/users", axum::routing::get(list_users))
//     .merge(swagger);
```

## Using OpenApiBuilder

`OpenApiBuilder` covers the `info` block only. To add paths, use utoipa derive and merge:

```rust
use rf_swagger::OpenApiBuilder;

let info_spec = OpenApiBuilder::new("My API", "1.0.0")
    .description("My great API")
    .contact("Alice", "alice@example.com")
    .license("MIT", "https://opensource.org/licenses/MIT")
    .build();
```

## Axum compatibility

`utoipa-swagger-ui 6.x` and `utoipa-redoc 3.x` target **axum 0.7**. The `SwaggerUi` and `Redoc`
types returned by this crate implement `Into<axum::Router>` for axum 0.7.

Upgrading to axum 0.8 integration requires `utoipa-swagger-ui 7.x` / `utoipa-redoc 4.x`,
which in turn require utoipa 5.x. That is tracked as a future upgrade.

## License

MIT OR Apache-2.0
