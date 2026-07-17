//! Field filtering and sparse fieldsets

use std::collections::HashSet;

/// Field filter for sparse fieldsets
#[derive(Debug, Clone, Default)]
pub struct FieldFilter {
    /// Fields to include
    pub include: Option<HashSet<String>>,
    /// Fields to exclude
    pub exclude: Option<HashSet<String>>,
}

impl FieldFilter {
    /// Create a new field filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Include specific fields
    pub fn include(mut self, fields: Vec<String>) -> Self {
        self.include = Some(fields.into_iter().collect());
        self
    }

    /// Exclude specific fields
    pub fn exclude(mut self, fields: Vec<String>) -> Self {
        self.exclude = Some(fields.into_iter().collect());
        self
    }

    /// Check if field should be included
    pub fn should_include(&self, field: &str) -> bool {
        // If exclude list exists and contains field, exclude it
        if let Some(ref exclude) = self.exclude {
            if exclude.contains(field) {
                return false;
            }
        }

        // If include list exists, only include if field is in list
        if let Some(ref include) = self.include {
            return include.contains(field);
        }

        // Default: include all fields
        true
    }

    /// Apply filter to JSON value
    pub fn apply(&self, mut value: serde_json::Value) -> serde_json::Value {
        if let serde_json::Value::Object(ref mut map) = value {
            map.retain(|k, _| self.should_include(k));
        }
        value
    }

    /// Parse from query string format: "field1,field2,field3"
    pub fn from_query_string(fields: &str) -> Self {
        let fields: Vec<String> = fields
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            include: Some(fields.into_iter().collect()),
            exclude: None,
        }
    }
}

/// Options for filtering
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Field filter
    pub fields: Option<FieldFilter>,
    /// Maximum nesting depth
    pub max_depth: usize,
}

impl FilterOptions {
    pub fn new() -> Self {
        Self {
            fields: None,
            max_depth: 5,
        }
    }

    pub fn with_fields(mut self, filter: FieldFilter) -> Self {
        self.fields = Some(filter);
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }
}

/// Parse include relationships from query string
pub fn parse_includes(includes: &str) -> Vec<String> {
    includes
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse field filters from query parameter
pub fn parse_field_filter(fields_param: &str) -> FieldFilter {
    FieldFilter::from_query_string(fields_param)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_include_filter() {
        let filter = FieldFilter::new().include(vec!["id".to_string(), "name".to_string()]);

        assert!(filter.should_include("id"));
        assert!(filter.should_include("name"));
        assert!(!filter.should_include("email"));
    }

    #[test]
    fn test_exclude_filter() {
        let filter = FieldFilter::new().exclude(vec!["password".to_string()]);

        assert!(filter.should_include("id"));
        assert!(filter.should_include("email"));
        assert!(!filter.should_include("password"));
    }

    #[test]
    fn test_apply_filter() {
        let filter = FieldFilter::new().include(vec!["id".to_string(), "name".to_string()]);

        let value = json!({
            "id": 1,
            "name": "Test",
            "email": "test@example.com",
            "password": "secret"
        });

        let filtered = filter.apply(value);
        let obj = filtered.as_object().unwrap();

        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("email"));
        assert!(!obj.contains_key("password"));
    }

    #[test]
    fn test_from_query_string() {
        let filter = FieldFilter::from_query_string("id,name,email");

        assert!(filter.should_include("id"));
        assert!(filter.should_include("name"));
        assert!(filter.should_include("email"));
        assert!(!filter.should_include("password"));
    }

    #[test]
    fn test_parse_includes() {
        let includes = parse_includes("posts,comments,profile");
        assert_eq!(includes.len(), 3);
        assert!(includes.contains(&"posts".to_string()));
    }
}
