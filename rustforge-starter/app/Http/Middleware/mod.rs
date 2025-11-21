/// HTTP Middleware
///
/// Middleware can inspect and filter HTTP requests entering your application.
///
/// Example middleware:
/// - Authentication
/// - CORS
/// - Rate Limiting
/// - Logging

use rf_web::{Request, Response, Next, Result};

/// Example Authentication Middleware
pub async fn authenticate(req: Request, next: Next) -> Result<Response> {
    // Example: Check if user is authenticated
    // if !req.user().is_some() {
    //     return Err(Error::Unauthorized);
    // }

    next.run(req).await
}

/// Example CORS Middleware
pub async fn cors(req: Request, next: Next) -> Result<Response> {
    let mut response = next.run(req).await?;

    // Add CORS headers
    response.header("Access-Control-Allow-Origin", "*");
    response.header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
    response.header("Access-Control-Allow-Headers", "Content-Type, Authorization");

    Ok(response)
}

/// Example Logging Middleware
pub async fn log_requests(req: Request, next: Next) -> Result<Response> {
    let method = req.method();
    let path = req.path();

    tracing::info!("{} {}", method, path);

    let response = next.run(req).await?;

    tracing::info!("Response: {}", response.status());

    Ok(response)
}
