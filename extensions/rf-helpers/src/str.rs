//! String manipulation helpers (Laravel Str::* equivalents)

use deunicode::deunicode;
use heck::{ToKebabCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};

/// Convert a string to slug format (hello-world)
pub fn slug(value: &str) -> String {
    deunicode(value)
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Convert a string to snake_case
pub fn snake(value: &str) -> String {
    value.to_snake_case()
}

/// Convert a string to camelCase
pub fn camel(value: &str) -> String {
    let pascal = value.to_upper_camel_case();
    if pascal.is_empty() {
        return pascal;
    }
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().chain(chars).collect(),
    }
}

/// Convert a string to PascalCase (StudlyCase in Laravel)
pub fn studly(value: &str) -> String {
    value.to_upper_camel_case()
}

/// Convert a string to kebab-case
pub fn kebab(value: &str) -> String {
    value.to_kebab_case()
}

/// Convert a string to Title Case
pub fn title(value: &str) -> String {
    value.to_title_case()
}

/// Pluralize a word (basic English rules)
pub fn plural(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }

    let lower = word.to_lowercase();

    // Common irregular plurals
    match lower.as_str() {
        "person" => return "people".to_string(),
        "child" => return "children".to_string(),
        "man" => return "men".to_string(),
        "woman" => return "women".to_string(),
        "foot" => return "feet".to_string(),
        "tooth" => return "teeth".to_string(),
        "mouse" => return "mice".to_string(),
        _ => {}
    }

    // Basic pluralization rules
    if lower.ends_with("s")
        || lower.ends_with("x")
        || lower.ends_with("z")
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        format!("{}es", word)
    } else if lower.ends_with("y") && !is_vowel(lower.chars().rev().nth(1).unwrap_or('a')) {
        let stem: String = word.chars().take(word.chars().count().saturating_sub(1)).collect();
        format!("{}ies", stem)
    } else if lower.ends_with("f") {
        let stem: String = word.chars().take(word.chars().count().saturating_sub(1)).collect();
        format!("{}ves", stem)
    } else if lower.ends_with("fe") {
        let stem: String = word.chars().take(word.chars().count().saturating_sub(2)).collect();
        format!("{}ves", stem)
    } else {
        format!("{}s", word)
    }
}

/// Singularize a word (basic English rules)
pub fn singular(word: &str) -> String {
    if word.is_empty() {
        return word.to_string();
    }

    let lower = word.to_lowercase();

    // Common irregular singulars
    match lower.as_str() {
        "people" => return "person".to_string(),
        "children" => return "child".to_string(),
        "men" => return "man".to_string(),
        "women" => return "woman".to_string(),
        "feet" => return "foot".to_string(),
        "teeth" => return "tooth".to_string(),
        "mice" => return "mouse".to_string(),
        _ => {}
    }

    // Basic singularization rules
    if lower.ends_with("ies") && word.chars().count() > 3 {
        let stem: String = word.chars().take(word.chars().count() - 3).collect();
        format!("{}y", stem)
    } else if lower.ends_with("ves") {
        if word.chars().count() > 3 {
            let stem: String = word.chars().take(word.chars().count() - 3).collect();
            format!("{}f", stem)
        } else {
            word.to_string()
        }
    } else if lower.ends_with("ses") || lower.ends_with("xes") || lower.ends_with("zes") {
        word.chars().take(word.chars().count().saturating_sub(2)).collect()
    } else if lower.ends_with("s") && word.chars().count() > 1 {
        word.chars().take(word.chars().count() - 1).collect()
    } else {
        word.to_string()
    }
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U')
}

/// Limit a string to a specified length
pub fn limit(value: &str, limit: usize, end: &str) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let truncated: String = value.chars().take(limit).collect();
    format!("{}{}", truncated, end)
}

/// Limit a string by word count
pub fn words(value: &str, words: usize, end: &str) -> String {
    let word_vec: Vec<&str> = value.split_whitespace().collect();
    if word_vec.len() <= words {
        return value.to_string();
    }
    let limited: Vec<&str> = word_vec.iter().take(words).copied().collect();
    format!("{}{}", limited.join(" "), end)
}

/// Check if a string contains a substring
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

/// Check if a string contains all needles
pub fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|&needle| haystack.contains(needle))
}

/// Check if a string starts with a prefix
pub fn starts_with(haystack: &str, needle: &str) -> bool {
    haystack.starts_with(needle)
}

/// Check if a string ends with a suffix
pub fn ends_with(haystack: &str, needle: &str) -> bool {
    haystack.ends_with(needle)
}

/// Get the portion of a string before a delimiter
pub fn before(subject: &str, search: &str) -> String {
    if let Some(pos) = subject.find(search) {
        subject[..pos].to_string()
    } else {
        subject.to_string()
    }
}

/// Get the portion of a string after a delimiter
pub fn after(subject: &str, search: &str) -> String {
    if let Some(pos) = subject.find(search) {
        subject[pos + search.len()..].to_string()
    } else {
        subject.to_string()
    }
}

/// Get the portion of a string between two delimiters
pub fn between(subject: &str, from: &str, to: &str) -> Option<String> {
    let start = subject.find(from)? + from.len();
    let end = subject[start..].find(to)?;
    Some(subject[start..start + end].to_string())
}

/// Replace the first occurrence of a string
pub fn replace_first(subject: &str, search: &str, replace: &str) -> String {
    if let Some(pos) = subject.find(search) {
        let before = &subject[..pos];
        let after = &subject[pos + search.len()..];
        format!("{}{}{}", before, replace, after)
    } else {
        subject.to_string()
    }
}

/// Replace the last occurrence of a string
pub fn replace_last(subject: &str, search: &str, replace: &str) -> String {
    if let Some(pos) = subject.rfind(search) {
        let before = &subject[..pos];
        let after = &subject[pos + search.len()..];
        format!("{}{}{}", before, replace, after)
    } else {
        subject.to_string()
    }
}

/// Uppercase the first character
pub fn ucfirst(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Lowercase the first character
pub fn lcfirst(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().chain(chars).collect(),
    }
}

/// Remove all whitespace from a string
pub fn remove_whitespace(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Generate a random string
pub fn random(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug() {
        assert_eq!(slug("Hello World"), "hello-world");
        assert_eq!(slug("Hello  World"), "hello-world");
        assert_eq!(slug("Hello_World"), "hello-world");
        assert_eq!(slug("café"), "cafe");
    }

    #[test]
    fn test_snake() {
        assert_eq!(snake("HelloWorld"), "hello_world");
        assert_eq!(snake("helloWorld"), "hello_world");
    }

    #[test]
    fn test_camel() {
        assert_eq!(camel("hello_world"), "helloWorld");
        assert_eq!(camel("HelloWorld"), "helloWorld");
    }

    #[test]
    fn test_studly() {
        assert_eq!(studly("hello_world"), "HelloWorld");
        assert_eq!(studly("hello-world"), "HelloWorld");
    }

    #[test]
    fn test_kebab() {
        assert_eq!(kebab("HelloWorld"), "hello-world");
        assert_eq!(kebab("helloWorld"), "hello-world");
    }

    #[test]
    fn test_title() {
        assert_eq!(title("hello world"), "Hello World");
    }

    #[test]
    fn test_plural() {
        assert_eq!(plural("user"), "users");
        assert_eq!(plural("person"), "people");
        assert_eq!(plural("child"), "children");
    }

    #[test]
    fn test_singular() {
        assert_eq!(singular("users"), "user");
        assert_eq!(singular("people"), "person");
        assert_eq!(singular("children"), "child");
    }

    #[test]
    fn test_limit() {
        assert_eq!(limit("Hello World", 5, "..."), "Hello...");
        assert_eq!(limit("Hi", 5, "..."), "Hi");
    }

    #[test]
    fn test_words() {
        assert_eq!(
            words("Hello beautiful world", 2, "..."),
            "Hello beautiful..."
        );
        assert_eq!(words("Hello world", 5, "..."), "Hello world");
    }

    #[test]
    fn test_contains() {
        assert!(contains("Hello World", "World"));
        assert!(!contains("Hello World", "Foo"));
    }

    #[test]
    fn test_contains_all() {
        assert!(contains_all("Hello World", &["Hello", "World"]));
        assert!(!contains_all("Hello World", &["Hello", "Foo"]));
    }

    #[test]
    fn test_starts_with() {
        assert!(starts_with("Hello World", "Hello"));
        assert!(!starts_with("Hello World", "World"));
    }

    #[test]
    fn test_ends_with() {
        assert!(ends_with("Hello World", "World"));
        assert!(!ends_with("Hello World", "Hello"));
    }

    #[test]
    fn test_before() {
        assert_eq!(before("Hello-World", "-"), "Hello");
        assert_eq!(before("Hello", "-"), "Hello");
    }

    #[test]
    fn test_after() {
        assert_eq!(after("Hello-World", "-"), "World");
        assert_eq!(after("Hello", "-"), "Hello");
    }

    #[test]
    fn test_between() {
        assert_eq!(
            between("Hello [World] Foo", "[", "]"),
            Some("World".to_string())
        );
        assert_eq!(between("Hello World", "[", "]"), None);
    }

    #[test]
    fn test_replace_first() {
        assert_eq!(replace_first("foo bar foo", "foo", "baz"), "baz bar foo");
    }

    #[test]
    fn test_replace_last() {
        assert_eq!(replace_last("foo bar foo", "foo", "baz"), "foo bar baz");
    }

    #[test]
    fn test_ucfirst() {
        assert_eq!(ucfirst("hello"), "Hello");
        assert_eq!(ucfirst("Hello"), "Hello");
    }

    #[test]
    fn test_lcfirst() {
        assert_eq!(lcfirst("Hello"), "hello");
        assert_eq!(lcfirst("hello"), "hello");
    }

    #[test]
    fn test_remove_whitespace() {
        assert_eq!(remove_whitespace("Hello World"), "HelloWorld");
        assert_eq!(remove_whitespace("  H e l l o  "), "Hello");
    }

    #[test]
    fn test_random() {
        let s1 = random(10);
        let s2 = random(10);
        assert_eq!(s1.len(), 10);
        assert_eq!(s2.len(), 10);
        // Should be different (with very high probability)
        assert_ne!(s1, s2);
    }
}
