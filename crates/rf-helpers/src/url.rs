//! URL generation helpers

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::collections::HashMap;
use url::Url;

/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

/// Generate a URL from a path
pub fn url(path: &str, base_url: Option<&str>) -> String {
    let base = base_url.unwrap_or("http://localhost:3000");
    let path = if path.starts_with('/') {
        path
    } else {
        &format!("/{}", path)
    };
    format!("{}{}", base, path)
}

/// Generate a secure URL (HTTPS)
pub fn secure_url(path: &str, base_url: Option<&str>) -> String {
    let base = base_url.unwrap_or("https://localhost:3000");
    let path = if path.starts_with('/') {
        path
    } else {
        &format!("/{}", path)
    };
    format!("{}{}", base, path)
}

/// Generate a URL to an asset
pub fn asset(path: &str, base_url: Option<&str>) -> String {
    url(&format!("/assets/{}", path.trim_start_matches('/')), base_url)
}

/// Generate a URL to a secure asset
pub fn secure_asset(path: &str, base_url: Option<&str>) -> String {
    secure_url(&format!("/assets/{}", path.trim_start_matches('/')), base_url)
}

/// Generate a named route URL (placeholder - requires route registry)
pub fn route(name: &str, params: HashMap<String, String>) -> String {
    // This would typically look up the route from a registry
    // For now, return a placeholder
    let mut path = format!("/routes/{}", name);
    if !params.is_empty() {
        let query: Vec<String> = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, utf8_percent_encode(v, FRAGMENT)))
            .collect();
        path = format!("{}?{}", path, query.join("&"));
    }
    path
}

/// Generate a previous URL (back)
pub fn previous(default: &str) -> String {
    // This would typically read from request headers or session
    // For now, return default
    default.to_string()
}

/// URL encode a string
pub fn encode(value: &str) -> String {
    utf8_percent_encode(value, FRAGMENT).to_string()
}

/// URL decode a string
pub fn decode(value: &str) -> Result<String, String> {
    percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .map(|s| s.to_string())
        .map_err(|e| e.to_string())
}

/// Build a query string from parameters
pub fn build_query(params: HashMap<String, String>) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Parse a query string into parameters
pub fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.split('=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((
                decode(key).unwrap_or_default(),
                decode(value).unwrap_or_default(),
            ))
        })
        .collect()
}

/// Get the current URL (placeholder - requires request context)
pub fn current() -> String {
    // This would typically read from the current request
    "/".to_string()
}

/// Get the full URL with query string (placeholder)
pub fn full() -> String {
    // This would typically read from the current request
    "/".to_string()
}

/// Check if the current URL matches a pattern (placeholder)
pub fn is_url(pattern: &str) -> bool {
    // This would typically compare against the current request URL
    current().contains(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url() {
        assert_eq!(url("/users", None), "http://localhost:3000/users");
        assert_eq!(url("users", None), "http://localhost:3000/users");
        assert_eq!(
            url("/users", Some("http://example.com")),
            "http://example.com/users"
        );
    }

    #[test]
    fn test_secure_url() {
        assert_eq!(secure_url("/users", None), "https://localhost:3000/users");
        assert_eq!(
            secure_url("/users", Some("https://example.com")),
            "https://example.com/users"
        );
    }

    #[test]
    fn test_asset() {
        assert_eq!(asset("css/app.css", None), "http://localhost:3000/assets/css/app.css");
        assert_eq!(asset("/css/app.css", None), "http://localhost:3000/assets/css/app.css");
    }

    #[test]
    fn test_encode() {
        assert_eq!(encode("hello world"), "hello%20world");
        assert_eq!(encode("hello<>world"), "hello%3C%3Eworld");
    }

    #[test]
    fn test_decode() {
        assert_eq!(decode("hello%20world").unwrap(), "hello world");
        assert_eq!(decode("hello%3C%3Eworld").unwrap(), "hello<>world");
    }

    #[test]
    fn test_build_query() {
        let mut params = HashMap::new();
        params.insert("foo".to_string(), "bar".to_string());
        params.insert("baz".to_string(), "qux".to_string());
        let query = build_query(params);
        // Order is not guaranteed, so check both possibilities
        assert!(query == "foo=bar&baz=qux" || query == "baz=qux&foo=bar");
    }

    #[test]
    fn test_parse_query() {
        let params = parse_query("foo=bar&baz=qux");
        assert_eq!(params.get("foo"), Some(&"bar".to_string()));
        assert_eq!(params.get("baz"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_route() {
        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());
        let url = route("user.show", params);
        assert!(url.contains("/routes/user.show"));
        assert!(url.contains("id=123"));
    }
}
