// Integration probe: security_headers
// Adapted from sandbox/probes/security_headers/src/main.rs
// Verifies the rf-web security-headers middleware layer.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use rf_web::{security_headers_layer, HstsConfig, SecurityHeadersConfig};
use tower::ServiceExt; // oneshot

fn get_request(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

#[tokio::test]
async fn test_security_headers() {
    // 1. Router WITH the layer — default config injects three secure headers
    let secured = Router::new()
        .route("/", get(|| async { "OK" }))
        .layer(security_headers_layer(SecurityHeadersConfig::default()));

    let resp = secured.clone().oneshot(get_request("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let xcto = resp
        .headers()
        .get("x-content-type-options")
        .and_then(|v| v.to_str().ok())
        .expect("X-Content-Type-Options must be present");
    assert_eq!(xcto, "nosniff");

    let resp = secured.clone().oneshot(get_request("/")).await.unwrap();
    let xfo = resp
        .headers()
        .get("x-frame-options")
        .and_then(|v| v.to_str().ok())
        .expect("X-Frame-Options must be present");
    assert_eq!(xfo, "DENY");

    let resp = secured.clone().oneshot(get_request("/")).await.unwrap();
    let rp = resp
        .headers()
        .get("referrer-policy")
        .and_then(|v| v.to_str().ok())
        .expect("Referrer-Policy must be present");
    assert_eq!(rp, "no-referrer");

    // 2. Router WITHOUT the layer — security headers must NOT appear (opt-in)
    let plain: Router = Router::new().route("/", get(|| async { "OK" }));
    let resp = plain.oneshot(get_request("/")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().get("x-content-type-options").is_none());
    assert!(resp.headers().get("x-frame-options").is_none());
    assert!(resp.headers().get("referrer-policy").is_none());

    // 3. HSTS absent by default; present when configured
    let resp = secured.clone().oneshot(get_request("/")).await.unwrap();
    assert!(resp.headers().get("strict-transport-security").is_none());

    let hsts_config = SecurityHeadersConfig::new().hsts(HstsConfig {
        max_age_secs: 31_536_000,
        include_subdomains: true,
        preload: false,
    });
    let hsts_app = Router::new()
        .route("/", get(|| async { "OK" }))
        .layer(security_headers_layer(hsts_config));
    let resp = hsts_app.oneshot(get_request("/")).await.unwrap();
    let hsts = resp
        .headers()
        .get("strict-transport-security")
        .and_then(|v| v.to_str().ok())
        .expect("HSTS must be present when configured");
    assert!(hsts.contains("max-age=31536000"), "got: {hsts}");
    assert!(hsts.contains("includeSubDomains"), "got: {hsts}");

    // 4. CSP absent by default; present when configured
    let resp = secured.clone().oneshot(get_request("/")).await.unwrap();
    assert!(resp.headers().get("content-security-policy").is_none());

    let csp_config = SecurityHeadersConfig::new().content_security_policy("default-src 'self'");
    let csp_app = Router::new()
        .route("/", get(|| async { "OK" }))
        .layer(security_headers_layer(csp_config));
    let resp = csp_app.oneshot(get_request("/")).await.unwrap();
    let csp = resp
        .headers()
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok())
        .expect("CSP must be present when configured");
    assert_eq!(csp, "default-src 'self'");

    // 5. Builder: disable X-Frame-Options; other headers still present
    let no_xfo_config = SecurityHeadersConfig::new().no_x_frame_options();
    let no_xfo_app = Router::new()
        .route("/", get(|| async { "OK" }))
        .layer(security_headers_layer(no_xfo_config));
    let resp = no_xfo_app.oneshot(get_request("/")).await.unwrap();
    assert!(resp.headers().get("x-frame-options").is_none());
    assert!(resp.headers().get("x-content-type-options").is_some());

    // 6. Builder: customise X-Frame-Options to SAMEORIGIN
    let sameorigin_config = SecurityHeadersConfig::new().x_frame_options("SAMEORIGIN");
    let sameorigin_app = Router::new()
        .route("/", get(|| async { "OK" }))
        .layer(security_headers_layer(sameorigin_config));
    let resp = sameorigin_app.oneshot(get_request("/")).await.unwrap();
    let xfo_custom = resp
        .headers()
        .get("x-frame-options")
        .and_then(|v| v.to_str().ok())
        .expect("X-Frame-Options must be present");
    assert_eq!(xfo_custom, "SAMEORIGIN");
}
