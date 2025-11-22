//! Naming convention utilities for scaffolding
//!
//! Provides utilities for converting names between different conventions:
//! - PascalCase (UserController)
//! - snake_case (user_controller)
//! - kebab-case (user-controller)
//! - Pluralization (user -> users)

/// Naming convention helper
pub struct NamingConvention;

impl NamingConvention {
    /// Create new naming convention helper
    pub fn new() -> Self {
        Self
    }

    /// Convert to PascalCase
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.to_pascal_case("user_controller"), "UserController");
    /// assert_eq!(nc.to_pascal_case("user-service"), "UserService");
    /// ```
    pub fn to_pascal_case(&self, name: &str) -> String {
        name.split(&['_', '-'][..])
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    /// Convert to snake_case
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.to_snake_case("UserController"), "user_controller");
    /// assert_eq!(nc.to_snake_case("HTTPClient"), "http_client");
    /// ```
    pub fn to_snake_case(&self, name: &str) -> String {
        // Handle already snake_case or kebab-case
        if name.contains('_') || name.contains('-') {
            return name.replace('-', "_").to_lowercase();
        }

        // Handle PascalCase/camelCase
        let mut result = String::new();
        let mut prev_lowercase = false;
        let mut prev_uppercase = false;

        for (i, ch) in name.chars().enumerate() {
            if ch.is_uppercase() {
                // Add underscore before uppercase if:
                // - Previous char was lowercase, or
                // - This is not the first char and we have multiple consecutive uppercase (like "HTTPClient")
                if i > 0
                    && (prev_lowercase
                        || (prev_uppercase
                            && name.chars().nth(i + 1).map_or(false, |c| c.is_lowercase())))
                {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap());
                prev_uppercase = true;
                prev_lowercase = false;
            } else {
                result.push(ch);
                prev_lowercase = true;
                prev_uppercase = false;
            }
        }

        result
    }

    /// Convert to kebab-case
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.to_kebab_case("UserController"), "user-controller");
    /// assert_eq!(nc.to_kebab_case("user_service"), "user-service");
    /// ```
    pub fn to_kebab_case(&self, name: &str) -> String {
        self.to_snake_case(name).replace('_', "-")
    }

    /// Pluralize a word (simple English rules)
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.pluralize("user"), "users");
    /// assert_eq!(nc.pluralize("class"), "classes");
    /// assert_eq!(nc.pluralize("category"), "categories");
    /// ```
    pub fn pluralize(&self, word: &str) -> String {
        if word.is_empty() {
            return word.to_string();
        }

        let lower = word.to_lowercase();

        // Irregular plurals
        let irregular = [
            ("person", "people"),
            ("child", "children"),
            ("man", "men"),
            ("woman", "women"),
            ("tooth", "teeth"),
            ("foot", "feet"),
            ("mouse", "mice"),
            ("goose", "geese"),
        ];

        for (singular, plural) in &irregular {
            if lower == *singular {
                return if word.chars().next().unwrap().is_uppercase() {
                    plural
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .chain(plural.chars().skip(1))
                        .collect()
                } else {
                    plural.to_string()
                };
            }
        }

        // Already plural (heuristic)
        if lower.ends_with("s") && !lower.ends_with("ss") {
            return word.to_string();
        }

        // Special endings
        if lower.ends_with("ch")
            || lower.ends_with("sh")
            || lower.ends_with("ss")
            || lower.ends_with('x')
            || lower.ends_with('z')
        {
            return format!("{}es", word);
        }

        if lower.ends_with('y') && lower.len() > 1 {
            let before_y = lower.chars().nth(lower.len() - 2).unwrap();
            if !"aeiou".contains(before_y) {
                // consonant + y -> ies
                return format!("{}ies", &word[..word.len() - 1]);
            }
        }

        if lower.ends_with("fe") {
            return format!("{}ves", &word[..word.len() - 2]);
        }

        if lower.ends_with('f') {
            return format!("{}ves", &word[..word.len() - 1]);
        }

        // Default: add 's'
        format!("{}s", word)
    }

    /// Singularize a word (simple English rules)
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.singularize("users"), "user");
    /// assert_eq!(nc.singularize("classes"), "class");
    /// assert_eq!(nc.singularize("categories"), "category");
    /// ```
    pub fn singularize(&self, word: &str) -> String {
        if word.is_empty() {
            return word.to_string();
        }

        let lower = word.to_lowercase();

        // Irregular plurals (reversed)
        let irregular = [
            ("people", "person"),
            ("children", "child"),
            ("men", "man"),
            ("women", "woman"),
            ("teeth", "tooth"),
            ("feet", "foot"),
            ("mice", "mouse"),
            ("geese", "goose"),
        ];

        for (plural, singular) in &irregular {
            if lower == *plural {
                return if word.chars().next().unwrap().is_uppercase() {
                    singular
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .chain(singular.chars().skip(1))
                        .collect()
                } else {
                    singular.to_string()
                };
            }
        }

        // Special endings
        if lower.ends_with("ies") && lower.len() > 3 {
            return format!("{}y", &word[..word.len() - 3]);
        }

        if lower.ends_with("ves") && lower.len() > 3 {
            return format!("{}f", &word[..word.len() - 3]);
        }

        if lower.ends_with("ses") && lower.len() > 3 {
            return word[..word.len() - 2].to_string();
        }

        if lower.ends_with("ches")
            || lower.ends_with("shes")
            || lower.ends_with("xes")
            || lower.ends_with("zes")
        {
            return word[..word.len() - 2].to_string();
        }

        if lower.ends_with('s') && !lower.ends_with("ss") {
            return word[..word.len() - 1].to_string();
        }

        word.to_string()
    }

    /// Extract base name from suffixed name
    ///
    /// # Example
    /// ```
    /// use rf_scaffold::naming::NamingConvention;
    ///
    /// let nc = NamingConvention::new();
    /// assert_eq!(nc.extract_base("UserController"), "User");
    /// assert_eq!(nc.extract_base("UserService"), "User");
    /// assert_eq!(nc.extract_base("User"), "User");
    /// ```
    pub fn extract_base(&self, name: &str) -> String {
        let suffixes = [
            "Controller",
            "Service",
            "Model",
            "Repository",
            "Factory",
            "Seeder",
        ];

        for suffix in &suffixes {
            if name.ends_with(suffix) && name.len() > suffix.len() {
                return name[..name.len() - suffix.len()].to_string();
            }
        }

        name.to_string()
    }
}

impl Default for NamingConvention {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_pascal_case() {
        let nc = NamingConvention::new();

        assert_eq!(nc.to_pascal_case("user"), "User");
        assert_eq!(nc.to_pascal_case("user_controller"), "UserController");
        assert_eq!(nc.to_pascal_case("user-service"), "UserService");
        assert_eq!(nc.to_pascal_case("http_client"), "HttpClient");
    }

    #[test]
    fn test_to_snake_case() {
        let nc = NamingConvention::new();

        assert_eq!(nc.to_snake_case("User"), "user");
        assert_eq!(nc.to_snake_case("UserController"), "user_controller");
        assert_eq!(nc.to_snake_case("HTTPClient"), "http_client");
        assert_eq!(nc.to_snake_case("user-service"), "user_service");
    }

    #[test]
    fn test_to_kebab_case() {
        let nc = NamingConvention::new();

        assert_eq!(nc.to_kebab_case("User"), "user");
        assert_eq!(nc.to_kebab_case("UserController"), "user-controller");
        assert_eq!(nc.to_kebab_case("user_service"), "user-service");
    }

    #[test]
    fn test_pluralize() {
        let nc = NamingConvention::new();

        assert_eq!(nc.pluralize("user"), "users");
        assert_eq!(nc.pluralize("class"), "classes");
        assert_eq!(nc.pluralize("category"), "categories");
        assert_eq!(nc.pluralize("child"), "children");
        assert_eq!(nc.pluralize("person"), "people");
        assert_eq!(nc.pluralize("box"), "boxes");
        assert_eq!(nc.pluralize("knife"), "knives");
    }

    #[test]
    fn test_singularize() {
        let nc = NamingConvention::new();

        assert_eq!(nc.singularize("users"), "user");
        assert_eq!(nc.singularize("classes"), "class");
        assert_eq!(nc.singularize("categories"), "category");
        assert_eq!(nc.singularize("children"), "child");
        assert_eq!(nc.singularize("people"), "person");
        assert_eq!(nc.singularize("boxes"), "box");
    }

    #[test]
    fn test_extract_base() {
        let nc = NamingConvention::new();

        assert_eq!(nc.extract_base("UserController"), "User");
        assert_eq!(nc.extract_base("UserService"), "User");
        assert_eq!(nc.extract_base("UserModel"), "User");
        assert_eq!(nc.extract_base("User"), "User");
    }
}
