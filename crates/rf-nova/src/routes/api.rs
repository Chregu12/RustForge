//! API response types for Nova
//!
//! Standard JSON response formats.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standard API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub errors: Option<Vec<String>>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            errors: None,
        }
    }

    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.into()),
            errors: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.into()),
            errors: None,
        }
    }

    pub fn error_with_details(message: impl Into<String>, errors: Vec<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.into()),
            errors: Some(errors),
        }
    }
}

/// Resource list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListResponse {
    pub resources: Vec<ResourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub name: String,
    pub uri_key: String,
    pub label: String,
    pub plural_label: String,
    pub group: Option<String>,
}

/// Dashboard list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardListResponse {
    pub dashboards: Vec<DashboardInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardInfo {
    pub name: String,
    pub uri_key: String,
}

/// Config response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub name: String,
    pub logo: Option<String>,
    pub theme: String,
    pub primary_color: String,
    pub global_search: bool,
    pub per_page_options: Vec<u64>,
    pub default_per_page: u64,
}
