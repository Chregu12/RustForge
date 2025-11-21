//! Form Request Validation Example
//!
//! This example demonstrates Laravel-like form request validation.

use axum::{
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use rf_validation::{
    form_request::{FormRequest, FormRequestError, FormRequestResult, Validated, ValidationRules},
    rules::{EmailRule, MinLengthRule, RequiredRule},
    validator::Rule,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
struct CreateUserRequest {
    email: String,
    password: String,
    name: String,
    age: Option<i32>,
}

#[async_trait::async_trait]
impl FormRequest for CreateUserRequest {
    type Validated = Self;

    fn rules(&self) -> ValidationRules {
        let mut rules: ValidationRules = HashMap::new();

        rules.insert(
            "email",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(EmailRule),
            ],
        );

        rules.insert(
            "password",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinLengthRule::new(8)),
            ],
        );

        rules.insert(
            "name",
            vec![
                Box::new(RequiredRule) as Box<dyn Rule>,
                Box::new(MinLengthRule::new(2)),
            ],
        );

        rules
    }

    fn messages(&self) -> HashMap<&'static str, &'static str> {
        let mut messages = HashMap::new();
        messages.insert("email.required", "Please provide an email address");
        messages.insert("email.email", "Please provide a valid email address");
        messages.insert("password.required", "Password is required");
        messages.insert("password.min_length", "Password must be at least 8 characters");
        messages.insert("name.required", "Name is required");
        messages.insert("name.min_length", "Name must be at least 2 characters");
        messages
    }

    fn authorize(&self) -> bool {
        // You could check user permissions here
        // For example: user.can("create-users")
        true
    }

    async fn validate(self) -> FormRequestResult<Self::Validated> {
        // Custom validation logic can go here
        // For now, we'll just return the request as validated

        // Check age if provided
        if let Some(age) = self.age {
            if age < 18 {
                return Err(FormRequestError::ValidationFailed(
                    rf_validation::error::ValidationErrors {
                        errors: {
                            let mut errors = HashMap::new();
                            errors.insert(
                                "age".to_string(),
                                vec![rf_validation::error::FieldError {
                                    code: "min_age".to_string(),
                                    message: "You must be at least 18 years old".to_string(),
                                    params: HashMap::new(),
                                }],
                            );
                            errors
                        },
                    },
                ));
            }
        }

        Ok(self)
    }

    fn prepare_for_validation(&mut self) {
        // Clean up data before validation
        self.email = self.email.trim().to_lowercase();
        self.name = self.name.trim().to_string();
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateUserRequest {
    email: Option<String>,
    name: Option<String>,
}

#[async_trait::async_trait]
impl FormRequest for UpdateUserRequest {
    type Validated = Self;

    fn rules(&self) -> ValidationRules {
        let mut rules: ValidationRules = HashMap::new();

        // Optional fields, but if provided, must be valid
        if self.email.is_some() {
            rules.insert("email", vec![Box::new(EmailRule) as Box<dyn Rule>]);
        }

        if self.name.is_some() {
            rules.insert("name", vec![Box::new(MinLengthRule::new(2)) as Box<dyn Rule>]);
        }

        rules
    }

    async fn validate(self) -> FormRequestResult<Self::Validated> {
        Ok(self)
    }
}

#[derive(Serialize)]
struct UserResponse {
    id: i32,
    email: String,
    name: String,
}

async fn create_user(Validated(request): Validated<CreateUserRequest>) -> impl IntoResponse {
    // At this point, the request has been:
    // 1. Authorized
    // 2. Validated according to rules
    // 3. Prepared (trimmed, normalized)

    // In a real application, you would save to database
    let user = UserResponse {
        id: 1,
        email: request.email,
        name: request.name,
    };

    Json(user)
}

async fn update_user(Validated(request): Validated<UpdateUserRequest>) -> impl IntoResponse {
    // Update user in database
    let user = UserResponse {
        id: 1,
        email: request.email.unwrap_or_else(|| "existing@example.com".to_string()),
        name: request.name.unwrap_or_else(|| "Existing Name".to_string()),
    };

    Json(user)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/users", post(create_user))
        .route("/users/1", post(update_user));

    println!("Server running on http://localhost:3000");
    println!("\nExample requests:");
    println!("\n1. Valid request:");
    println!(r#"curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{{"email": "user@example.com", "password": "password123", "name": "John Doe", "age": 25}}'"#);

    println!("\n2. Invalid request (short password):");
    println!(r#"curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{{"email": "user@example.com", "password": "short", "name": "John Doe"}}'"#);

    println!("\n3. Invalid request (bad email):");
    println!(r#"curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{{"email": "not-an-email", "password": "password123", "name": "John Doe"}}'"#);

    println!("\n4. Update request:");
    println!(r#"curl -X POST http://localhost:3000/users/1 \
  -H "Content-Type: application/json" \
  -d '{{"name": "Jane Doe"}}'"#);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
