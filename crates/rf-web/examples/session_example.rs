//! Session Management Example
//!
//! This example demonstrates how to use sessions for user state management.

use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use rf_web::session::{CookieSessionDriver, Session, SessionConfig, SessionMiddleware};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn show_login_form(session: Session) -> Html<String> {
    // Get flash messages
    let mut session = session;
    let error_message = session
        .get_flash("error")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // Get old input for form repopulation
    let old_username = session.old("username").unwrap_or_default();

    let error_html = if !error_message.is_empty() {
        format!(r#"<div class="error">{}</div>"#, error_message)
    } else {
        String::new()
    };

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Login</title>
    <style>
        .error {{ color: red; padding: 10px; margin: 10px 0; }}
        .success {{ color: green; padding: 10px; margin: 10px 0; }}
    </style>
</head>
<body>
    <h1>Login</h1>
    {}
    <form method="POST" action="/login">
        <div>
            <label>Username:</label>
            <input type="text" name="username" value="{}" required>
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
        error_html, old_username
    );

    Html(html)
}

async fn process_login(mut session: Session, Form(form): Form<LoginForm>) -> impl IntoResponse {
    // Simple authentication (in real app, check against database)
    if form.username == "admin" && form.password == "password" {
        // Store user data in session
        session.put("user_id", 1);
        session.put("username", form.username.clone());
        session.put("authenticated", true);

        // Flash success message
        session.flash("success", "Login successful!");

        // Regenerate session ID for security
        let _ = session.regenerate().await;

        Redirect::to("/dashboard").into_response()
    } else {
        // Flash error message and old input
        session.flash("error", "Invalid credentials");
        let mut old_input = std::collections::HashMap::new();
        old_input.insert("username".to_string(), form.username);
        session.flash_input(old_input);

        Redirect::to("/login").into_response()
    }
}

async fn show_dashboard(session: Session) -> impl IntoResponse {
    // Check if user is authenticated
    let authenticated = session.get_as::<bool>("authenticated").unwrap_or(false);

    if !authenticated {
        return Redirect::to("/login").into_response();
    }

    let username = session.get_as::<String>("username").unwrap_or_default();

    // Get flash message
    let mut session = session;
    let success_message = session
        .get_flash("success")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    let success_html = if !success_message.is_empty() {
        format!(r#"<div class="success">{}</div>"#, success_message)
    } else {
        String::new()
    };

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Dashboard</title>
    <style>
        .success {{ color: green; padding: 10px; margin: 10px 0; }}
    </style>
</head>
<body>
    <h1>Dashboard</h1>
    {}
    <p>Welcome, {}!</p>
    <p>Session ID: {}</p>
    <a href="/logout">Logout</a>
</body>
</html>
    "#,
        success_html,
        username,
        session.id()
    );

    Html(html).into_response()
}

async fn logout(mut session: Session) -> impl IntoResponse {
    // Invalidate session
    let _ = session.invalidate().await;

    Redirect::to("/login")
}

#[tokio::main]
async fn main() {
    // Create session driver
    let driver = Arc::new(CookieSessionDriver::new());

    // Create session configuration
    let session_config = SessionConfig::new()
        .cookie_name("my_session")
        .lifetime(3600) // 1 hour
        .secure(false) // Set to true in production with HTTPS
        .http_only(true);

    // Create session middleware
    let session_middleware = SessionMiddleware::with_config(driver, session_config);

    // Create router with session support
    let app = Router::new()
        .route("/login", get(show_login_form))
        .route("/login", post(process_login))
        .route("/dashboard", get(show_dashboard))
        .route("/logout", get(logout));

    println!("Server running on http://localhost:3000");
    println!("Visit http://localhost:3000/login");
    println!("Use username: admin, password: password");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
