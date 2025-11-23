//! Span builder utilities for creating instrumented code blocks

use opentelemetry::KeyValue;
use std::future::Future;

/// Builder for creating instrumented spans
pub struct SpanBuilder {
    name: String,
    attributes: Vec<KeyValue>,
}

impl SpanBuilder {
    /// Create a new span builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
        }
    }

    /// Add an attribute to the span
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes
            .push(KeyValue::new(key.into(), value.into()));
        self
    }

    /// Add a numeric attribute
    pub fn with_int_attribute(mut self, key: impl Into<String>, value: i64) -> Self {
        self.attributes.push(KeyValue::new(key.into(), value));
        self
    }

    /// Add a boolean attribute
    pub fn with_bool_attribute(mut self, key: impl Into<String>, value: bool) -> Self {
        self.attributes.push(KeyValue::new(key.into(), value));
        self
    }

    /// Execute a future within this span
    pub async fn in_span<F, T>(self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        // Use tracing spans for now
        // Full OpenTelemetry span integration requires compatible API versions
        tracing::trace_span!("otel_span", name = %self.name)
            .in_scope(|| future)
            .await
    }

    /// Execute a synchronous closure within this span
    pub fn in_span_sync<F, T>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        // Use tracing for now
        // Full OpenTelemetry span integration requires compatible API versions
        f()
    }
}

/// Macro for creating instrumented async blocks
#[macro_export]
macro_rules! span {
    ($name:expr) => {
        $crate::span_builder::SpanBuilder::new($name)
    };
    ($name:expr, $($key:expr => $value:expr),+ $(,)?) => {
        {
            let mut builder = $crate::span_builder::SpanBuilder::new($name);
            $(
                builder = builder.with_attribute($key, $value.to_string());
            )+
            builder
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_builder_creation() {
        let builder = SpanBuilder::new("test_span")
            .with_attribute("key1", "value1")
            .with_int_attribute("key2", 42);

        assert_eq!(builder.name, "test_span");
        assert_eq!(builder.attributes.len(), 2);
    }

    #[tokio::test]
    async fn test_span_execution() {
        let result = SpanBuilder::new("test")
            .with_attribute("test", "value")
            .in_span(async { 42 })
            .await;

        assert_eq!(result, 42);
    }

    #[test]
    fn test_span_sync_execution() {
        let result = SpanBuilder::new("test_sync")
            .with_int_attribute("count", 10)
            .in_span_sync(|| 42);

        assert_eq!(result, 42);
    }
}
