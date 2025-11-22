use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tera::{Function, Result as TeraResult};

/// Function for generating CSRF tokens
#[derive(Clone)]
pub struct CsrfTokenFunction {
    token: Arc<RwLock<String>>,
}

impl CsrfTokenFunction {
    pub fn new() -> Self {
        Self {
            token: Arc::new(RwLock::new(Self::generate_token())),
        }
    }

    pub fn set_token(&self, token: impl Into<String>) {
        if let Ok(mut t) = self.token.write() {
            *t = token.into();
        }
    }

    fn generate_token() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("csrf_{}", now)
    }
}

impl Function for CsrfTokenFunction {
    fn call(&self, _args: &HashMap<String, Value>) -> TeraResult<Value> {
        let token = self
            .token
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read token: {}", e)))?;
        Ok(Value::String(token.clone()))
    }
}

/// Function for getting the authenticated user
#[derive(Clone)]
pub struct AuthFunction {
    user: Arc<RwLock<Option<Value>>>,
}

impl AuthFunction {
    pub fn new() -> Self {
        Self {
            user: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_user(&self, user: Value) {
        if let Ok(mut u) = self.user.write() {
            *u = Some(user);
        }
    }

    pub fn clear_user(&self) {
        if let Ok(mut u) = self.user.write() {
            *u = None;
        }
    }
}

impl Function for AuthFunction {
    fn call(&self, _args: &HashMap<String, Value>) -> TeraResult<Value> {
        let user = self
            .user
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read user: {}", e)))?;
        Ok(user.clone().unwrap_or(Value::Null))
    }
}

/// Function for getting old input values (after validation errors)
#[derive(Clone)]
pub struct OldFunction {
    old_input: Arc<RwLock<HashMap<String, Value>>>,
}

impl OldFunction {
    pub fn new() -> Self {
        Self {
            old_input: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_old_input(&self, key: impl Into<String>, value: Value) {
        if let Ok(mut input) = self.old_input.write() {
            input.insert(key.into(), value);
        }
    }

    pub fn set_all_old_input(&self, input: HashMap<String, Value>) {
        if let Ok(mut old) = self.old_input.write() {
            *old = input;
        }
    }

    pub fn clear_old_input(&self) {
        if let Ok(mut input) = self.old_input.write() {
            input.clear();
        }
    }
}

impl Function for OldFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("'key' parameter is required"))?;

        let input = self
            .old_input
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read old input: {}", e)))?;

        Ok(input
            .get(key)
            .cloned()
            .unwrap_or(Value::String(String::new())))
    }
}

/// Function for getting validation errors
#[derive(Clone)]
pub struct ErrorFunction {
    errors: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ErrorFunction {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_error(&self, field: impl Into<String>, error: impl Into<String>) {
        if let Ok(mut errors) = self.errors.write() {
            errors
                .entry(field.into())
                .or_insert_with(Vec::new)
                .push(error.into());
        }
    }

    pub fn set_errors(&self, errors: HashMap<String, Vec<String>>) {
        if let Ok(mut errs) = self.errors.write() {
            *errs = errors;
        }
    }

    pub fn clear_errors(&self) {
        if let Ok(mut errors) = self.errors.write() {
            errors.clear();
        }
    }
}

impl Function for ErrorFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("'field' parameter is required"))?;

        let errors = self
            .errors
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read errors: {}", e)))?;

        if let Some(field_errors) = errors.get(field) {
            if let Some(first_error) = field_errors.first() {
                return Ok(Value::String(first_error.clone()));
            }
        }

        Ok(Value::Null)
    }
}

/// Function for getting all validation errors for a field
#[derive(Clone)]
pub struct ErrorsFunction {
    errors: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ErrorsFunction {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_errors(&self, errors: HashMap<String, Vec<String>>) {
        if let Ok(mut errs) = self.errors.write() {
            *errs = errors;
        }
    }
}

impl Function for ErrorsFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("'field' parameter is required"))?;

        let errors = self
            .errors
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read errors: {}", e)))?;

        if let Some(field_errors) = errors.get(field) {
            let error_values: Vec<Value> = field_errors
                .iter()
                .map(|e| Value::String(e.clone()))
                .collect();
            return Ok(Value::Array(error_values));
        }

        Ok(Value::Array(Vec::new()))
    }
}

/// Function for checking if a field has errors
#[derive(Clone)]
pub struct HasErrorFunction {
    errors: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl HasErrorFunction {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_errors(&self, errors: HashMap<String, Vec<String>>) {
        if let Ok(mut errs) = self.errors.write() {
            *errs = errors;
        }
    }
}

impl Function for HasErrorFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let field = args
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("'field' parameter is required"))?;

        let errors = self
            .errors
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read errors: {}", e)))?;

        let has_error = errors.get(field).map_or(false, |errs| !errs.is_empty());
        Ok(Value::Bool(has_error))
    }
}

/// Function for getting flash messages
#[derive(Clone)]
pub struct FlashFunction {
    flash: Arc<RwLock<HashMap<String, String>>>,
}

impl FlashFunction {
    pub fn new() -> Self {
        Self {
            flash: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_flash(&self, key: impl Into<String>, message: impl Into<String>) {
        if let Ok(mut flash) = self.flash.write() {
            flash.insert(key.into(), message.into());
        }
    }

    pub fn clear_flash(&self) {
        if let Ok(mut flash) = self.flash.write() {
            flash.clear();
        }
    }
}

impl Function for FlashFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("'key' parameter is required"))?;

        let flash = self
            .flash
            .read()
            .map_err(|e| tera::Error::msg(format!("Failed to read flash: {}", e)))?;

        Ok(flash
            .get(key)
            .map(|v| Value::String(v.clone()))
            .unwrap_or(Value::Null))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_function() {
        let func = CsrfTokenFunction::new();
        func.set_token("test_token");

        let result = func.call(&HashMap::new()).unwrap();
        assert_eq!(result.as_str().unwrap(), "test_token");
    }

    #[test]
    fn test_auth_function() {
        let func = AuthFunction::new();

        // No user set
        let result = func.call(&HashMap::new()).unwrap();
        assert!(result.is_null());

        // User set
        let user = serde_json::json!({"id": 1, "name": "John"});
        func.set_user(user.clone());

        let result = func.call(&HashMap::new()).unwrap();
        assert_eq!(result["name"].as_str().unwrap(), "John");
    }

    #[test]
    fn test_old_function() {
        let func = OldFunction::new();
        func.set_old_input("email", Value::String("test@example.com".to_string()));

        let mut args = HashMap::new();
        args.insert("key".to_string(), Value::String("email".to_string()));

        let result = func.call(&args).unwrap();
        assert_eq!(result.as_str().unwrap(), "test@example.com");
    }

    #[test]
    fn test_error_function() {
        let func = ErrorFunction::new();
        func.set_error("email", "Invalid email format");

        let mut args = HashMap::new();
        args.insert("field".to_string(), Value::String("email".to_string()));

        let result = func.call(&args).unwrap();
        assert_eq!(result.as_str().unwrap(), "Invalid email format");
    }

    #[test]
    fn test_has_error_function() {
        let func = HasErrorFunction::new();
        let mut errors = HashMap::new();
        errors.insert("email".to_string(), vec!["Invalid email".to_string()]);
        func.set_errors(errors);

        let mut args = HashMap::new();
        args.insert("field".to_string(), Value::String("email".to_string()));

        let result = func.call(&args).unwrap();
        assert!(result.as_bool().unwrap());

        args.insert("field".to_string(), Value::String("name".to_string()));
        let result = func.call(&args).unwrap();
        assert!(!result.as_bool().unwrap());
    }

    #[test]
    fn test_flash_function() {
        let func = FlashFunction::new();
        func.set_flash("success", "Operation completed successfully");

        let mut args = HashMap::new();
        args.insert("key".to_string(), Value::String("success".to_string()));

        let result = func.call(&args).unwrap();
        assert_eq!(result.as_str().unwrap(), "Operation completed successfully");
    }
}
