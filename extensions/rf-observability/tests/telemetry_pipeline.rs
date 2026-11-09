//! Verify that the installed tracer sends spans to an OTLP gRPC collector.
//!
//! Runs in its own test binary so it owns a clean global tracing-subscriber
//! slot (unit tests that install their own subscriber live in a separate
//! process).

use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, Tracer};
use opentelemetry_proto::tonic::collector::trace::v1::{
    trace_service_server::{TraceService, TraceServiceServer},
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use std::time::Duration;
use tokio::{net::TcpListener, sync::mpsc};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

use rf_observability::OtelConfig;

struct Collector(mpsc::UnboundedSender<ExportTraceServiceRequest>);

#[tonic::async_trait]
impl TraceService for Collector {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
        self.0
            .send(request.into_inner())
            .map_err(|_| Status::internal("collector receiver closed"))?;
        Ok(Response::new(ExportTraceServiceResponse::default()))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn init_telemetry_installs_global_tracer_and_exports_spans() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (tx, mut rx) = mpsc::unbounded_channel();
    let server = tokio::spawn(
        Server::builder()
            .add_service(TraceServiceServer::new(Collector(tx)))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    let config = OtelConfig {
        enabled: true,
        endpoint,
        use_tls: false,
        sample_rate: 1.0,
        timeout_seconds: 3,
        batch_config: Default::default(),
    };

    rf_observability::init_telemetry(&config).expect("init_telemetry should build a real pipeline");

    // The global provider must now be the real SDK provider: a tracer obtained
    // from it produces spans with a valid (sampled, non-empty) span context.
    let tracer = global::tracer("telemetry-pipeline-probe");

    let recorded_valid = tracer.in_span("probe-span", |cx| {
        let span_ctx = cx.span().span_context().clone();
        // A no-op tracer returns an invalid/empty span context; a real SDK
        // tracer with a ratio(1.0) sampler always yields a valid one.
        assert!(
            span_ctx.is_valid(),
            "expected a valid span context from a real installed tracer provider"
        );
        assert!(
            span_ctx.is_sampled(),
            "span should be sampled with sample_rate = 1.0"
        );
        assert_ne!(
            span_ctx.trace_id(),
            opentelemetry::trace::TraceId::INVALID,
            "trace id must be non-zero for a real tracer"
        );
        assert_ne!(
            span_ctx.span_id(),
            opentelemetry::trace::SpanId::INVALID,
            "span id must be non-zero for a real tracer"
        );
        span_ctx.is_valid()
    });

    assert!(recorded_valid, "a real span should have been recorded");

    rf_observability::shutdown_telemetry().await.unwrap();
    rf_observability::shutdown_telemetry().await.unwrap();

    let batch = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("collector did not receive the flushed batch")
        .expect("collector stopped before receiving the batch");
    assert!(batch
        .resource_spans
        .iter()
        .flat_map(|resource| &resource.scope_spans)
        .flat_map(|scope| &scope.spans)
        .any(|span| span.name == "probe-span"));
    server.abort();
}
