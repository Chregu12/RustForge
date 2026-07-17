# Foundry HTTP Client

Guzzle-style HTTP client with request builder for Foundry Core.

## Features

- **Request Builder Pattern**: Fluent API for building requests
- **Multiple Methods**: GET, POST, PUT, PATCH, DELETE, HEAD
- **JSON & Form Data**: Easy request body handling
- **Authentication**: Bearer, Basic, Custom headers
- **Retry Logic**: Automatic retry with backoff
- **Timeout Control**: Request-level timeouts
- **Middleware Support**: Request/response processing
- **SSL Verification**: Certificate validation control

## Quick Start

```rust
use foundry_http_client::HttpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = HttpClient::new();

    // Simple GET request
    let response = client
        .get("https://api.example.com/users")
        .header("Accept", "application/json")
        .send()
        .await?;

    let users: Vec<User> = response.json().await?;

    Ok(())
}
```

## CLI Commands

```bash
# Make HTTP request
foundry http:request GET https://api.example.com/users
foundry http:request POST https://api.example.com/users --body '{"name":"John"}'
```

## Request Methods

```rust
let client = HttpClient::new();

// GET
let response = client.get("https://api.example.com/users").send().await?;

// POST with JSON
let response = client
    .post("https://api.example.com/users")
    .json(&user)?
    .send()
    .await?;

// PUT
let response = client
    .put("https://api.example.com/users/1")
    .json(&updated_user)?
    .send()
    .await?;

// DELETE
let response = client
    .delete("https://api.example.com/users/1")
    .send()
    .await?;
```

## Authentication

```rust
// Bearer token
let response = client
    .get("https://api.example.com/protected")
    .bearer_auth("your_token_here")
    .send()
    .await?;

// Basic authentication
let response = client
    .get("https://api.example.com/protected")
    .basic_auth("username", "password")
    .send()
    .await?;
```

## Query Parameters

```rust
let response = client
    .get("https://api.example.com/users")
    .query("page", "1")
    .query("per_page", "50")
    .query("status", "active")
    .send()
    .await?;
```

## Request Body

```rust
// JSON
let user = User { name: "Alice".to_string() };
let response = client
    .post("https://api.example.com/users")
    .json(&user)?
    .send()
    .await?;

// Form data
let mut form = HashMap::new();
form.insert("name".to_string(), "Alice".to_string());
form.insert("email".to_string(), "alice@example.com".to_string());

let response = client
    .post("https://api.example.com/users")
    .form(form)
    .send()
    .await?;

// Text
let response = client
    .post("https://api.example.com/webhook")
    .text("Custom payload")
    .send()
    .await?;
```

## Response Handling

```rust
let response = client.get("https://api.example.com/users").send().await?;

// Check status
if response.is_success() {
    println!("Success!");
}

// Get as JSON
let users: Vec<User> = response.json().await?;

// Get as text
let body = response.text().await?;

// Get as bytes
let bytes = response.bytes().await?;
```

## Retry Configuration

```rust
use foundry_http_client::{RetryConfig, HttpClientBuilder};
use std::time::Duration;

let retry_config = RetryConfig::new(5)
    .with_delay(Duration::from_secs(1))
    .with_backoff(2.0);

let client = HttpClientBuilder::new()
    .retry_config(retry_config)
    .build()?;
```

## Client Configuration

```rust
use foundry_http_client::HttpClientBuilder;
use std::time::Duration;

let client = HttpClientBuilder::new()
    .timeout(30)
    .user_agent("MyApp/1.0")
    .default_header("X-API-Key", "secret")
    .verify_ssl(true)
    .build()?;
```

## Middleware

```rust
use foundry_http_client::middleware::LoggingMiddleware;

let client = HttpClientBuilder::new()
    .middleware(Box::new(LoggingMiddleware))
    .build()?;
```
