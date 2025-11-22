//! SPA (Single Page Application) authentication support
//!
//! Provides CSRF-protected cookie authentication for SPAs

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use rand::Rng;

/// Generate CSRF token for SPA authentication
pub fn generate_csrf_token() -> String {
    use rand::distributions::Alphanumeric;
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

/// Handler to set CSRF cookie for SPA
///
/// Laravel equivalent: `/sanctum/csrf-cookie`
///
/// # Example
///
/// ```rust,ignore
/// use rf_sanctum::spa::sanctum_csrf_cookie;
///
/// let app = Router::new()
///     .route("/sanctum/csrf-cookie", get(sanctum_csrf_cookie));
/// ```
pub async fn sanctum_csrf_cookie() -> impl IntoResponse {
    let token = generate_csrf_token();

    // Create XSRF-TOKEN cookie (readable by JavaScript)
    let cookie = Cookie::build(("XSRF-TOKEN", token.clone()))
        .path("/")
        .http_only(false) // Must be readable by JavaScript
        .same_site(SameSite::Lax)
        .secure(true) // Should be true in production
        .to_string();

    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)])
}

/// Middleware to verify CSRF token for SPA requests
pub async fn verify_csrf_token(
    cookies: axum_extra::extract::CookieJar,
    headers: axum::http::HeaderMap,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<Response, StatusCode> {
    // Get CSRF token from cookie
    let cookie_token = cookies
        .get("XSRF-TOKEN")
        .map(|c| c.value())
        .ok_or(StatusCode::FORBIDDEN)?;

    // Get CSRF token from header
    let header_token = headers
        .get("X-XSRF-TOKEN")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    // Verify tokens match
    if cookie_token != header_token {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_csrf_token() {
        let token = generate_csrf_token();
        assert_eq!(token.len(), 40);
    }

    #[tokio::test]
    async fn test_sanctum_csrf_cookie() {
        let response = sanctum_csrf_cookie().await.into_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }
}
