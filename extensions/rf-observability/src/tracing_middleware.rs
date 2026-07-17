//! Distributed tracing middleware with W3C Trace Context propagation

use axum::{
    extract::Request,
    http::{header, HeaderMap},
    middleware::Next,
    response::Response,
};
use opentelemetry::{
    propagation::{Extractor, Injector, TextMapPropagator},
    Context,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::time::Instant;

use crate::metrics::METRICS;

/// HTTP headers extractor for W3C Trace Context
struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// HTTP headers injector for W3C Trace Context
struct HeaderInjector<'a>(&'a mut HeaderMap);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = header::HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

/// Tracing middleware for HTTP requests
pub struct TracingMiddleware;

impl TracingMiddleware {
    /// Extract trace context from request headers (W3C Trace Context)
    pub fn extract_trace_context(headers: &HeaderMap) -> Context {
        let propagator = TraceContextPropagator::new();
        let extractor = HeaderExtractor(headers);
        propagator.extract(&extractor)
    }

    /// Inject trace context into response headers
    pub fn inject_trace_context(ctx: &Context, headers: &mut HeaderMap) {
        let propagator = TraceContextPropagator::new();
        let mut injector = HeaderInjector(headers);
        propagator.inject_context(ctx, &mut injector);
    }

    /// Create middleware handler for Axum
    pub async fn middleware(request: Request, next: Next) -> Response {
        let start = Instant::now();
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        // Extract parent trace context from headers
        let _parent_ctx = Self::extract_trace_context(request.headers());

        // Note: Full span integration requires compatible OpenTelemetry versions
        // For now, we focus on metrics and basic tracing logging

        // Increment in-flight counter
        METRICS.http_requests_in_flight.inc();

        // Execute request
        let response = next.run(request).await;

        // Decrement in-flight counter
        METRICS.http_requests_in_flight.dec();

        let duration = start.elapsed();
        let status = response.status();

        // Record metrics
        METRICS.record_http_request(&method, &path, status.as_u16(), duration);

        response
    }
}

/// Extract trace ID from current context for logging correlation.
///
/// Returns `Some(hex_trace_id)` when called inside a tracing span that has an
/// active OpenTelemetry context (i.e. the tracing subscriber includes a
/// `tracing_opentelemetry` layer).  Returns `None` when called outside any
/// span or when no OTel layer is wired, which is the honest fail-open
/// behaviour rather than fabricating an id.
pub fn current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let ctx = tracing::Span::current().context();
    let sc = ctx.span().span_context().clone();
    if sc.is_valid() {
        Some(sc.trace_id().to_string())
    } else {
        None
    }
}

/// Extract span ID from the current OpenTelemetry context.
///
/// Returns `Some(hex_span_id)` when there is an active OTel-instrumented span
/// on the current thread, `None` otherwise.
pub fn current_span_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let ctx = tracing::Span::current().context();
    let sc = ctx.span().span_context().clone();
    if sc.is_valid() {
        Some(sc.span_id().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use opentelemetry::trace::TraceContextExt;

    /// Verify that `current_trace_id` / `current_span_id` return a real hex id
    /// when the current thread is inside a span backed by an OTel tracer, and
    /// return `None` when outside such a span (honest fail-open).
    #[test]
    fn test_trace_id_correlation_inside_otel_span() {
        use opentelemetry_sdk::trace::TracerProvider as SdkProvider;
        use tracing_subscriber::layer::SubscriberExt;

        // Outside any otel-wired span: should be None, not fabricated.
        assert_eq!(
            current_trace_id(),
            None,
            "current_trace_id should be None outside an OTel span"
        );

        // Build a real in-process tracer provider (no exporter needed for id tests).
        let provider = SdkProvider::builder().build();
        let tracer = opentelemetry::trace::TracerProvider::tracer(&provider, "test-tracer");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let sub = tracing_subscriber::registry().with(otel_layer);

        // Scope the subscriber to this test thread only.
        let _guard = tracing::subscriber::set_default(sub);

        let span = tracing::info_span!("test_correlation_span");
        let _enter = span.enter();

        let trace_id = current_trace_id();
        assert!(
            trace_id.is_some(),
            "current_trace_id() must return Some inside an OTel-instrumented span, got None"
        );
        let tid = trace_id.unwrap();
        assert_eq!(tid.len(), 32, "trace_id should be 32 hex chars, got {:?}", tid);

        let span_id = current_span_id();
        assert!(
            span_id.is_some(),
            "current_span_id() must return Some inside an OTel-instrumented span, got None"
        );
        let sid = span_id.unwrap();
        assert_eq!(sid.len(), 16, "span_id should be 16 hex chars, got {:?}", sid);
    }

    #[test]
    fn test_extract_trace_context() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );

        let ctx = TracingMiddleware::extract_trace_context(&headers);
        let span = ctx.span();
        assert!(span.span_context().is_valid());
    }

    #[test]
    fn test_inject_trace_context() {
        let mut headers = HeaderMap::new();
        let ctx = Context::current();

        TracingMiddleware::inject_trace_context(&ctx, &mut headers);

        // Headers may or may not be injected depending on span validity
        // This test just ensures no panic occurs
    }

    #[test]
    fn test_header_extractor() {
        let mut headers = HeaderMap::new();
        headers.insert("test-key", HeaderValue::from_static("test-value"));

        let extractor = HeaderExtractor(&headers);
        assert_eq!(extractor.get("test-key"), Some("test-value"));
        assert_eq!(extractor.get("missing"), None);
    }
}
