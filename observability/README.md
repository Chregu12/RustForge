# RustForge Observability Stack

Complete monitoring and observability infrastructure for RustForge applications.

## Overview

This directory contains configuration files for a complete observability stack including:

- **OpenTelemetry Collector**: Aggregates and processes telemetry data
- **Jaeger**: Distributed tracing visualization
- **Prometheus**: Metrics collection and storage
- **Grafana**: Metrics visualization and dashboards
- **Alertmanager**: Alert routing and notification management
- **Node Exporter**: System-level metrics

## Quick Start

### 1. Start the Monitoring Stack

```bash
cd observability/
docker-compose up -d
```

### 2. Access the Services

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://localhost:3001 | admin / admin |
| Prometheus | http://localhost:9090 | - |
| Jaeger UI | http://localhost:16686 | - |
| Alertmanager | http://localhost:9093 | - |

### 3. Configure Your Application

Set these environment variables:

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export PROMETHEUS_ENDPOINT=/metrics
export RUST_LOG=info
```

### 4. View Metrics and Traces

1. **Metrics**: Open Grafana at http://localhost:3001
   - Navigate to Dashboards → RustForge Performance
   - View real-time metrics, latencies, and error rates

2. **Traces**: Open Jaeger at http://localhost:16686
   - Select "rustforge" service
   - Search for traces by operation or tags
   - Analyze request flows and dependencies

3. **Alerts**: Open Prometheus at http://localhost:9090
   - Navigate to Alerts to see active/pending alerts
   - Check alerting rules status

## Directory Structure

```
observability/
├── docker-compose.yml              # Complete stack orchestration
├── otel-collector-config.yml       # OpenTelemetry Collector config
├── prometheus/
│   ├── prometheus.yml              # Prometheus server config
│   └── alerts.yml                  # Alerting rules
├── grafana/
│   └── rustforge-dashboard.json    # Pre-built dashboard
└── alertmanager/
    └── config.yml                  # Alert routing config
```

## Configuration Files

### Prometheus (`prometheus/prometheus.yml`)

Configures:
- Scrape intervals (15s)
- Target endpoints (application, node exporter, postgres, redis)
- Alert evaluation rules
- Remote write endpoints (optional)

**Key scrape targets:**
- `rustforge`: Your application metrics (port 3000)
- `prometheus`: Self-monitoring
- `node`: System metrics
- `postgres-exporter`: Database metrics
- `redis-exporter`: Cache metrics

### Alerting Rules (`prometheus/alerts.yml`)

Pre-configured alerts:
- **Critical**: Error rates, application down, connection pool exhaustion
- **Warning**: Slow requests, high memory/CPU, queue backlogs

**Alert groups:**
- `rustforge_alerts`: Application-specific alerts
- `rustforge_recording_rules`: Pre-computed metrics for dashboards

### Grafana Dashboard (`grafana/rustforge-dashboard.json`)

Includes panels for:
- Command execution rates and duration (p50, p95)
- HTTP request rates and latency
- Error rates and trends
- Cache hit rates
- Database connection pools and query performance
- Queue sizes and processing times
- Application uptime and in-flight requests

### OpenTelemetry Collector (`otel-collector-config.yml`)

Pipelines:
- **Traces**: OTLP → Batch → Jaeger
- **Metrics**: OTLP → Batch → Prometheus

Processors:
- Memory limiter (512MB)
- Batch processor (1024 batch size)
- Resource attribute injection
- PII scrubbing (user agents, IPs)

### Alertmanager (`alertmanager/config.yml`)

Configure notification channels:
- Slack webhooks
- PagerDuty integration
- Email SMTP
- Custom receivers per team/component

## Customization

### Adding Custom Metrics

In your application:

```rust
use foundry_observability::METRICS;

// Custom counter
METRICS.commands_total
    .with_label_values(&["my_command", "success"])
    .inc();
```

Then add to Grafana dashboard:
```promql
rate(rustforge_commands_total{command_name="my_command"}[5m])
```

### Creating Custom Alerts

Add to `prometheus/alerts.yml`:

```yaml
- alert: CustomAlert
  expr: your_metric > threshold
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Custom alert triggered"
    description: "Your metric exceeded threshold"
```

### Adding Dashboard Panels

1. Open Grafana at http://localhost:3001
2. Edit the RustForge dashboard
3. Add new panel with PromQL query
4. Export JSON and update `grafana/rustforge-dashboard.json`

## Production Deployment

### Scaling Considerations

1. **Prometheus Retention**:
   ```yaml
   command:
     - '--storage.tsdb.retention.time=30d'
     - '--storage.tsdb.retention.size=50GB'
   ```

2. **OpenTelemetry Collector**:
   - Deploy multiple collectors behind load balancer
   - Increase memory limits for high traffic
   - Use persistent queue for reliability

3. **Grafana**:
   - Use external database (PostgreSQL)
   - Enable LDAP/OAuth authentication
   - Set up provisioning for automated dashboard deployment

### High Availability

- Run multiple Prometheus replicas with Thanos
- Use Alertmanager clustering
- Deploy OTel Collectors in HA mode
- Set up Grafana load balancing

### Security

1. **Enable authentication** on all services
2. **Use TLS** for production endpoints
3. **Restrict network access** with firewall rules
4. **Rotate credentials** regularly
5. **Enable audit logging**

### Cloud Provider Integration

#### AWS
- Use Amazon Managed Prometheus (AMP)
- Send traces to AWS X-Ray
- Use CloudWatch for logs

#### GCP
- Use Google Cloud Monitoring
- Send traces to Cloud Trace
- Use Cloud Logging

#### Azure
- Use Azure Monitor
- Send traces to Application Insights

## Troubleshooting

### Metrics Not Appearing

1. Check Prometheus targets: http://localhost:9090/targets
2. Verify application `/metrics` endpoint is accessible
3. Check OTel Collector health: http://localhost:13133
4. Review Prometheus logs: `docker-compose logs prometheus`

### Traces Not Showing in Jaeger

1. Verify OTLP endpoint is reachable from application
2. Check OTel Collector logs: `docker-compose logs otel-collector`
3. Verify Jaeger is receiving data: `docker-compose logs jaeger`
4. Check trace sampling rate (`OTEL_TRACES_SAMPLER_ARG`)

### Alerts Not Firing

1. Verify alerting rules are loaded in Prometheus
2. Check alert conditions are met
3. Verify Alertmanager is configured
4. Test notification channels

### High Memory Usage

1. Reduce Prometheus retention period
2. Lower scrape intervals for non-critical targets
3. Increase OTel Collector batch size
4. Disable unused metrics exporters

## Monitoring Best Practices

### Golden Signals

Monitor these key metrics:
1. **Latency**: Request duration (p50, p95, p99)
2. **Traffic**: Requests per second
3. **Errors**: Error rate and count
4. **Saturation**: Resource utilization (CPU, memory, connections)

### SLI/SLO Tracking

Define Service Level Indicators:
```promql
# Availability SLI (target: 99.9%)
sum(rate(rustforge_http_requests_total{status!~"5.."}[30d]))
/
sum(rate(rustforge_http_requests_total[30d]))
```

### Cardinality Management

Avoid high-cardinality labels:
- ❌ User IDs, request IDs, timestamps
- ✅ HTTP methods, status codes, endpoints

### Dashboard Organization

1. **Overview**: Key metrics, SLO compliance
2. **Performance**: Latencies, throughput
3. **Errors**: Error rates, failed requests
4. **Infrastructure**: CPU, memory, disk, network
5. **Business**: Domain-specific metrics

## Resources

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)

## Support

For issues or questions:
1. Check application logs
2. Review service health checks
3. Consult monitoring stack logs
4. Refer to component documentation
