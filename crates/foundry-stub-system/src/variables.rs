use heck::{
    ToKebabCase, ToLowerCamelCase, ToPascalCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Case conversion utilities
pub struct CaseConverter;

impl CaseConverter {
    /// Convert to PascalCase (StudlyCase)
    pub fn studly(s: &str) -> String {
        s.to_pascal_case()
    }

    /// Convert to snake_case
    pub fn snake(s: &str) -> String {
        s.to_snake_case()
    }

    /// Convert to kebab-case
    pub fn kebab(s: &str) -> String {
        s.to_kebab_case()
    }

    /// Convert to camelCase
    pub fn camel(s: &str) -> String {
        s.to_lower_camel_case()
    }

    /// Convert to SCREAMING_SNAKE_CASE
    pub fn screaming_snake(s: &str) -> String {
        s.to_shouty_snake_case()
    }

    /// Convert to Title Case
    pub fn title(s: &str) -> String {
        s.to_title_case()
    }

    /// Get plural form (simple implementation)
    pub fn plural(s: &str) -> String {
        if s.ends_with('y') && !s.ends_with("ay") && !s.ends_with("ey") && !s.ends_with("oy") && !s.ends_with("uy") {
            format!("{}ies", &s[..s.len() - 1])
        } else if s.ends_with('s') || s.ends_with("sh") || s.ends_with("ch") || s.ends_with('x') || s.ends_with('z') {
            format!("{}es", s)
        } else {
            format!("{}s", s)
        }
    }

    /// Get singular form (simple implementation)
    pub fn singular(s: &str) -> String {
        if s.ends_with("ies") {
            format!("{}y", &s[..s.len() - 3])
        } else if s.ends_with("es") {
            s[..s.len() - 2].to_string()
        } else if s.ends_with('s') && !s.ends_with("ss") {
            s[..s.len() - 1].to_string()
        } else {
            s.to_string()
        }
    }
}

/// Variables available for stub templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StubVariables {
    /// Original name
    pub name: String,
    /// Namespace/module path
    pub namespace: String,
    /// PascalCase version
    pub studly: String,
    /// snake_case version
    pub snake: String,
    /// kebab-case version
    pub kebab: String,
    /// camelCase version
    pub camel: String,
    /// Plural form
    pub plural: String,
    /// Singular form
    pub singular: String,
    /// Custom variables
    #[serde(flatten)]
    pub custom: HashMap<String, String>,
}

impl StubVariables {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let studly = CaseConverter::studly(&name);
        let snake = CaseConverter::snake(&name);
        let kebab = CaseConverter::kebab(&name);
        let camel = CaseConverter::camel(&name);
        let plural = CaseConverter::plural(&name);
        let singular = CaseConverter::singular(&name);

        Self {
            name: name.clone(),
            namespace: String::new(),
            studly,
            snake,
            kebab,
            camel,
            plural,
            singular,
            custom: HashMap::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Convert to a HashMap for template rendering
    pub fn to_context(&self) -> HashMap<String, String> {
        let mut context = HashMap::new();
        context.insert("name".to_string(), self.name.clone());
        context.insert("namespace".to_string(), self.namespace.clone());
        context.insert("studly".to_string(), self.studly.clone());
        context.insert("snake".to_string(), self.snake.clone());
        context.insert("kebab".to_string(), self.kebab.clone());
        context.insert("camel".to_string(), self.camel.clone());
        context.insert("plural".to_string(), self.plural.clone());
        context.insert("singular".to_string(), self.singular.clone());

        // Add custom variables
        for (key, value) in &self.custom {
            context.insert(key.clone(), value.clone());
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_conversions() {
        assert_eq!(CaseConverter::studly("user_profile"), "UserProfile");
        assert_eq!(CaseConverter::snake("UserProfile"), "user_profile");
        assert_eq!(CaseConverter::kebab("UserProfile"), "user-profile");
        assert_eq!(CaseConverter::camel("user_profile"), "userProfile");
        assert_eq!(CaseConverter::screaming_snake("UserProfile"), "USER_PROFILE");
        assert_eq!(CaseConverter::title("user_profile"), "User Profile");
    }

    #[test]
    fn test_plural_singular() {
        assert_eq!(CaseConverter::plural("user"), "users");
        assert_eq!(CaseConverter::plural("company"), "companies");
        assert_eq!(CaseConverter::plural("box"), "boxes");
        assert_eq!(CaseConverter::plural("child"), "childs"); // Simple implementation

        assert_eq!(CaseConverter::singular("users"), "user");
        assert_eq!(CaseConverter::singular("companies"), "company");
        assert_eq!(CaseConverter::singular("boxes"), "box");
    }

    #[test]
    fn test_stub_variables() {
        let vars = StubVariables::new("UserProfile")
            .with_namespace("app::models")
            .with_custom("table", "user_profiles");

        assert_eq!(vars.name, "UserProfile");
        assert_eq!(vars.namespace, "app::models");
        assert_eq!(vars.studly, "UserProfile");
        assert_eq!(vars.snake, "user_profile");
        assert_eq!(vars.kebab, "user-profile");
        assert_eq!(vars.camel, "userProfile");
        assert!(vars.custom.contains_key("table"));
    }

    #[test]
    fn test_variables_to_context() {
        let vars = StubVariables::new("Post")
            .with_custom("author", "admin");

        let context = vars.to_context();

        assert_eq!(context.get("name"), Some(&"Post".to_string()));
        assert_eq!(context.get("studly"), Some(&"Post".to_string()));
        assert_eq!(context.get("snake"), Some(&"post".to_string()));
        assert_eq!(context.get("author"), Some(&"admin".to_string()));
    }
}
