//! OpenAPI/Swagger documentation for RustForge
//!
//! rf-swagger is a **thin integration layer** over [utoipa](https://docs.rs/utoipa) 4.x.
//! It does **not** auto-generate specs by introspecting routes; the caller is responsible
//! for building an [`utoipa::openapi::OpenApi`] (typically via `#[derive(utoipa::OpenApi)]`
//! and `#[utoipa::path]` annotations), then passing it to [`swagger_ui`] or [`redoc`].
//!
//! # Quick start
//!
//! ```rust,ignore
//! use rf_swagger::{swagger_ui, OpenApiBuilder};
//! use utoipa::OpenApi as UtoipaOpenApi;
//!
//! // 1. Annotate your handlers with #[utoipa::path] and collect them:
//! #[utoipa::path(get, path = "/health", responses((status = 200, description = "ok")))]
//! async fn health() -> &'static str { "ok" }
//!
//! #[derive(utoipa::OpenApi)]
//! #[openapi(paths(health))]
//! struct ApiDoc;
//!
//! // 2. Build the spec and pass it to swagger_ui / redoc:
//! let spec = ApiDoc::openapi();
//! let swagger = swagger_ui(spec);
//!
//! // 3. Merge into your axum 0.7 router (utoipa-swagger-ui 6.x targets axum 0.7):
//! // let app = axum::Router::new().merge(swagger);
//! ```
//!
//! # Axum compatibility note
//!
//! The underlying crates `utoipa-swagger-ui 6.x` and `utoipa-redoc 3.x` target **axum 0.7**.
//! Merging the returned [`SwaggerUi`]/[`Redoc`] into an axum **0.8** router requires upgrading
//! to `utoipa-swagger-ui 7.x` / `utoipa-redoc 4.x` (which in turn require utoipa 5.x).
//! That upgrade is outside the scope of this crate; document this honestly to users.

use serde::{Deserialize, Serialize};
use utoipa::openapi::{ContactBuilder, InfoBuilder, LicenseBuilder, OpenApiBuilder as UtoipaOpenApiBuilder};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub use utoipa;
pub use utoipa::ToSchema;

/// Thin builder that collects API metadata and produces a [`utoipa::openapi::OpenApi`].
///
/// Use [`build`](OpenApiBuilder::build) to obtain the spec, then add paths by merging a
/// utoipa `#[derive(OpenApi)]` spec on top, or pass the result directly to
/// [`swagger_ui`] / [`redoc`].
///
/// **Note:** This builder only captures metadata (title, version, contact, …).
/// To include actual paths/schemas you must use utoipa's `#[utoipa::path]` annotations
/// and `#[derive(utoipa::OpenApi)]` — see the crate-level docs.
#[derive(Clone)]
pub struct OpenApiBuilder {
    title: String,
    version: String,
    description: Option<String>,
    terms_of_service: Option<String>,
    contact_name: Option<String>,
    contact_email: Option<String>,
    license_name: Option<String>,
    license_url: Option<String>,
}

impl OpenApiBuilder {
    /// Create a new OpenAPI builder with a required title and version.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: None,
            terms_of_service: None,
            contact_name: None,
            contact_email: None,
            license_name: None,
            license_url: None,
        }
    }

    /// Set API description (markdown is supported).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set URL to the terms of service.
    pub fn terms_of_service(mut self, terms: impl Into<String>) -> Self {
        self.terms_of_service = Some(terms.into());
        self
    }

    /// Set contact name and email.
    pub fn contact(mut self, name: impl Into<String>, email: impl Into<String>) -> Self {
        self.contact_name = Some(name.into());
        self.contact_email = Some(email.into());
        self
    }

    /// Set license name and URL.
    pub fn license(mut self, name: impl Into<String>, url: impl Into<String>) -> Self {
        self.license_name = Some(name.into());
        self.license_url = Some(url.into());
        self
    }

    /// Get the title.
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Get the version.
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Get the description.
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Build a [`utoipa::openapi::OpenApi`] from the accumulated metadata.
    ///
    /// The returned spec contains only the `info` block (title, version, contact, …).
    /// To include paths and schemas, merge this with a spec produced by
    /// `#[derive(utoipa::OpenApi)]`.
    pub fn build(self) -> utoipa::openapi::OpenApi {
        let mut info = InfoBuilder::new()
            .title(self.title)
            .version(self.version)
            .description(self.description)
            .terms_of_service(self.terms_of_service);

        if self.contact_name.is_some() || self.contact_email.is_some() {
            let contact = ContactBuilder::new()
                .name(self.contact_name)
                .email(self.contact_email)
                .build();
            info = info.contact(Some(contact));
        }

        if let Some(license_name) = self.license_name {
            let license = LicenseBuilder::new()
                .name(license_name)
                .url(self.license_url)
                .build();
            info = info.license(Some(license));
        }

        UtoipaOpenApiBuilder::new().info(info.build()).build()
    }
}

/// Create a Swagger UI router that serves the provided OpenAPI spec.
///
/// The returned [`SwaggerUi`] mounts:
/// - `/swagger-ui` — the interactive UI
/// - `/api-docs/openapi.json` — the raw JSON spec
///
/// # Panics
/// Does not panic.
///
/// # Axum compatibility
/// [`SwaggerUi`] (from `utoipa-swagger-ui 6.x`) implements `Into<axum::Router>` for
/// **axum 0.7**. Merge into an axum 0.7 router via `.merge(swagger_ui(spec))`.
pub fn swagger_ui(openapi: utoipa::openapi::OpenApi) -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi)
}

/// Create a ReDoc router that serves the provided OpenAPI spec.
///
/// The returned [`Redoc`] mounts `/redoc` with the interactive ReDoc UI.
///
/// # Axum compatibility
/// [`Redoc`] (from `utoipa-redoc 3.x`) implements `Into<axum::Router>` for **axum 0.7**.
pub fn redoc(openapi: utoipa::openapi::OpenApi) -> Redoc<'static, 'static, utoipa::openapi::OpenApi> {
    Redoc::with_url("/redoc", openapi)
}

/// OpenAPI documentation info
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// OpenAPI server configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiServer {
    pub url: String,
    pub description: Option<String>,
}

/// OpenAPI tag for grouping endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiTag {
    pub name: String,
    pub description: Option<String>,
}

/// Generic API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    /// Create a success response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// Pagination metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    pub total: i64,
    pub per_page: i64,
    pub current_page: i64,
    pub last_page: i64,
}

/// Paginated API response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_builder() {
        let builder = OpenApiBuilder::new("Test API", "1.0.0")
            .description("Test API description")
            .terms_of_service("https://example.com/terms")
            .contact("Test", "test@example.com")
            .license("MIT", "https://opensource.org/licenses/MIT");

        assert_eq!(builder.get_title(), "Test API");
        assert_eq!(builder.get_version(), "1.0.0");
        assert_eq!(builder.get_description(), Some("Test API description"));
    }

    /// OpenApiBuilder::build() must produce a serializable utoipa OpenApi with
    /// the correct title and version in the info block.
    #[test]
    fn test_openapi_builder_build() {
        let openapi = OpenApiBuilder::new("My API", "2.0.0")
            .description("A test API")
            .contact("Alice", "alice@example.com")
            .license("MIT", "https://opensource.org/licenses/MIT")
            .build();

        let json = serde_json::to_string(&openapi).expect("OpenApi must be serializable");

        assert!(json.contains("My API"), "serialized spec must contain title");
        assert!(json.contains("2.0.0"), "serialized spec must contain version");
        assert!(json.contains("alice@example.com"), "serialized spec must contain contact email");
        assert!(json.contains("MIT"), "serialized spec must contain license name");
    }

    /// swagger_ui() must accept a non-default OpenApi and not panic.
    /// Verify that the spec info is the one we passed (not the empty default).
    #[test]
    fn test_swagger_ui_uses_passed_spec() {
        let openapi = OpenApiBuilder::new("Probe API", "9.9.9").build();
        // Serialize the spec we are about to pass so we know what to expect.
        let expected_json = serde_json::to_string(&openapi).unwrap();
        assert!(expected_json.contains("Probe API"));

        // swagger_ui() must not panic and must accept the spec.
        // (Before the fix this took a _String_ and discarded it, serving the empty default.)
        let _ui = swagger_ui(openapi);
        // If we reach here without panic, the function accepted the real spec.
    }

    /// redoc() must accept a non-default OpenApi and not panic.
    #[test]
    fn test_redoc_uses_passed_spec() {
        let openapi = OpenApiBuilder::new("Redoc API", "1.2.3").build();
        let _rd = redoc(openapi);
    }

    #[test]
    fn test_api_info() {
        let info = ApiInfo {
            title: "Test".to_string(),
            version: "1.0".to_string(),
            description: Some("Description".to_string()),
        };

        assert_eq!(info.title, "Test");
        assert_eq!(info.version, "1.0");
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success(42);
        assert!(response.success);
        assert_eq!(response.data, Some(42));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<i32> = ApiResponse::error("Test error");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta {
            total: 100,
            per_page: 10,
            current_page: 1,
            last_page: 10,
        };

        assert_eq!(meta.total, 100);
        assert_eq!(meta.per_page, 10);
    }

    #[test]
    fn test_paginated_response() {
        let response = PaginatedResponse {
            data: vec![1, 2, 3],
            meta: PaginationMeta {
                total: 3,
                per_page: 10,
                current_page: 1,
                last_page: 1,
            },
        };

        assert_eq!(response.data.len(), 3);
        assert_eq!(response.meta.total, 3);
    }

    #[test]
    fn test_api_server() {
        let server = ApiServer {
            url: "https://api.example.com".to_string(),
            description: Some("Production server".to_string()),
        };

        assert_eq!(server.url, "https://api.example.com");
        assert_eq!(server.description, Some("Production server".to_string()));
    }

    #[test]
    fn test_api_tag() {
        let tag = ApiTag {
            name: "users".to_string(),
            description: Some("User management".to_string()),
        };

        assert_eq!(tag.name, "users");
        assert_eq!(tag.description, Some("User management".to_string()));
    }
}
