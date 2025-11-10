//! OpenAPI/Swagger documentation for RustForge
//!
//! This crate provides automatic API documentation generation with Swagger UI and ReDoc.

use axum::{
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa_redoc::{Redoc, Servable};

pub use utoipa;
pub use utoipa::ToSchema;

/// OpenAPI builder for creating API documentation
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
    /// Create a new OpenAPI builder
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

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set terms of service
    pub fn terms_of_service(mut self, terms: impl Into<String>) -> Self {
        self.terms_of_service = Some(terms.into());
        self
    }

    /// Set contact information
    pub fn contact(mut self, name: impl Into<String>, email: impl Into<String>) -> Self {
        self.contact_name = Some(name.into());
        self.contact_email = Some(email.into());
        self
    }

    /// Set license
    pub fn license(mut self, name: impl Into<String>, url: impl Into<String>) -> Self {
        self.license_name = Some(name.into());
        self.license_url = Some(url.into());
        self
    }

    /// Get the title
    pub fn get_title(&self) -> &str {
        &self.title
    }

    /// Get the version
    pub fn get_version(&self) -> &str {
        &self.version
    }

    /// Get the description
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Create Swagger UI router
pub fn swagger_ui<S>(openapi_json: String) -> SwaggerUi
where
    S: Clone + Send + Sync + 'static,
{
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", utoipa::openapi::OpenApi::default())
}

/// Create ReDoc router
pub fn redoc<S>(openapi_json: String) -> Redoc
where
    S: Clone + Send + Sync + 'static,
{
    Redoc::with_url("/redoc", utoipa::openapi::OpenApi::default())
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

/// Example API response
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
