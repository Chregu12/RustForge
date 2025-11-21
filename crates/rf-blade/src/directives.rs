//! Blade directives
//!
//! Built-in and custom directive handlers

use std::collections::HashMap;

/// Directive handler function
pub type DirectiveHandler = Box<dyn Fn(&str) -> String + Send + Sync>;

/// Directive registry
pub struct DirectiveRegistry {
    directives: HashMap<String, DirectiveHandler>,
}

impl DirectiveRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            directives: HashMap::new(),
        };

        // Register built-in directives
        registry.register_builtin();

        registry
    }

    /// Register a custom directive
    pub fn register<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.directives.insert(name.to_string(), Box::new(handler));
    }

    /// Execute a directive
    pub fn execute(&self, name: &str, args: &str) -> Option<String> {
        self.directives.get(name).map(|handler| handler(args))
    }

    /// Register built-in directives
    fn register_builtin(&mut self) {
        // @csrf - CSRF token field
        self.register("csrf", |_| {
            r#"<input type="hidden" name="_token" value="{{ csrf_token() }}">"#.to_string()
        });

        // @method - HTTP method spoofing
        self.register("method", |method| {
            format!(r#"<input type="hidden" name="_method" value="{}">"#, method)
        });

        // @json - Output JSON
        self.register("json", |var| {
            format!("{{{{ json_encode({}) }}}}", var)
        });

        // @dd - Dump and die
        self.register("dd", |var| {
            format!("{{{{ dd({}) }}}}", var)
        });

        // @dump - Dump variable
        self.register("dump", |var| {
            format!("{{{{ dump({}) }}}}", var)
        });

        // @env - Environment check
        self.register("env", |env| {
            format!("{{{{ env('{}') }}}}", env)
        });

        // @production - Show only in production
        self.register("production", |_| {
            "<!-- production -->".to_string()
        });

        // @error - Display validation error
        self.register("error", |field| {
            format!(r#"@if($errors->has('{}'))
    <div class="error">{{{{ $errors->first('{}') }}}}</div>
@endif"#, field, field)
        });
    }
}

impl Default for DirectiveRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_directive() {
        let registry = DirectiveRegistry::new();

        let output = registry.execute("csrf", "").unwrap();

        assert!(output.contains("_token"));
        assert!(output.contains("csrf_token()"));
    }

    #[test]
    fn test_method_directive() {
        let registry = DirectiveRegistry::new();

        let output = registry.execute("method", "PUT").unwrap();

        assert!(output.contains("_method"));
        assert!(output.contains("PUT"));
    }

    #[test]
    fn test_custom_directive() {
        let mut registry = DirectiveRegistry::new();

        registry.register("upper", |value| value.to_uppercase());

        let output = registry.execute("upper", "hello").unwrap();

        assert_eq!(output, "HELLO");
    }

    #[test]
    fn test_error_directive() {
        let registry = DirectiveRegistry::new();

        let output = registry.execute("error", "email").unwrap();

        assert!(output.contains("email"));
        assert!(output.contains("@if"));
        assert!(output.contains("$errors"));
    }
}
