//! CSRF Protection Example
//!
//! This example demonstrates how to use CSRF protection in a web application.

use axum::{
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use rf_web::csrf::{csrf_field, csrf_meta, csrf_token, CsrfConfig, CsrfLayer};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LoginForm {
    _token: String,
    username: String,
    password: String,
}

async fn show_login_form() -> Html<String> {
    let token = csrf_token();

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Login</title>
    {}
</head>
<body>
    <h1>Login</h1>
    <form method="POST" action="/login">
        {}
        <div>
            <label>Username:</label>
            <input type="text" name="username" required>
        </div>
        <div>
            <label>Password:</label>
            <input type="password" name="password" required>
        </div>
        <button type="submit">Login</button>
    </form>
</body>
</html>
    "#,
        csrf_meta(&token),
        csrf_field(&token)
    );

    Html(html)
}

async fn process_login(Form(form): Form<LoginForm>) -> impl IntoResponse {
    // In a real application, you would:
    // 1. Verify CSRF token against session
    // 2. Validate credentials
    // 3. Create session
    // 4. Redirect to dashboard

    (
        StatusCode::OK,
        format!("Login successful for user: {}", form.username),
    )
}

#[tokio::main]
async fn main() {
    // Create CSRF configuration
    let csrf_config = CsrfConfig::new()
        .exempt("/health") // Exempt health check
        .exempt("/metrics") // Exempt metrics endpoint
        .lifetime_hours(2) // Token lifetime: 2 hours
        .header_name("X-CSRF-TOKEN"); // Accept token from this header

    // Create router with CSRF protection
    let app = Router::new()
        .route("/login", get(show_login_form))
        .route("/login", post(process_login))
        .layer(CsrfLayer::with_config(csrf_config));

    println!("Server running on http://localhost:3000");
    println!("Visit http://localhost:3000/login to see CSRF protection in action");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
