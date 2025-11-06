# RustForge Production Deployment Checklist

This checklist ensures your RustForge application is production-ready.

## Pre-Deployment Checklist

### 1. Code Quality

- [ ] All tests passing (`cargo test --workspace --all-features`)
- [ ] Zero clippy warnings (`cargo clippy-strict`)
- [ ] Code properly formatted (`cargo fmt-check`)
- [ ] Documentation builds (`cargo doc-all`)
- [ ] No TODO/FIXME in critical paths
- [ ] No debug print statements in production code

### 2. Security

- [ ] Security audit passes (`cargo audit`)
- [ ] No known vulnerabilities in dependencies
- [ ] Secrets stored in environment variables (not in code)
- [ ] HTTPS/TLS configured
- [ ] CORS properly configured
- [ ] Rate limiting enabled
- [ ] SQL injection prevention verified
- [ ] XSS protection verified
- [ ] CSRF tokens implemented

### 3. Testing

- [ ] Unit tests > 80% coverage
- [ ] Integration tests passing
- [ ] E2E tests passing
- [ ] Load testing completed
- [ ] Security testing completed
- [ ] Database migration tested

### 4. Performance

- [ ] Benchmarks run and documented
- [ ] Performance targets met
- [ ] Memory usage acceptable (< 100MB under load)
- [ ] No memory leaks detected
- [ ] Database queries optimized
- [ ] Caching strategy implemented

### 5. Configuration

- [ ] `.env.example` up to date
- [ ] Environment-specific configs ready (dev, staging, prod)
- [ ] Database connection pooling configured
- [ ] Logging levels configured
- [ ] Feature flags documented
- [ ] Secrets management strategy defined

### 6. Database

- [ ] Migrations tested
- [ ] Rollback strategy defined
- [ ] Backups configured
- [ ] Connection pooling tuned
- [ ] Indexes optimized
- [ ] Query performance acceptable

### 7. Observability

- [ ] Prometheus metrics exposed
- [ ] Grafana dashboards created
- [ ] Log aggregation configured
- [ ] Error tracking setup (Sentry, etc.)
- [ ] Health check endpoint working
- [ ] Alerts configured

### 8. Infrastructure

- [ ] Dockerfile builds successfully
- [ ] Docker image optimized (< 100MB)
- [ ] Docker Compose tested
- [ ] Kubernetes manifests ready (if applicable)
- [ ] CI/CD pipeline configured
- [ ] Auto-scaling configured
- [ ] Load balancer configured

### 9. Documentation

- [ ] README.md complete
- [ ] API documentation generated
- [ ] Deployment guide written
- [ ] Runbook created
- [ ] Architecture diagrams updated
- [ ] Changelog maintained

### 10. Legal & Compliance

- [ ] License files present
- [ ] Privacy policy reviewed
- [ ] GDPR compliance (if applicable)
- [ ] Terms of service ready
- [ ] Copyright notices correct

---

## Deployment Steps

### Step 1: Pre-deployment Verification

```bash
# Run production check
./scripts/production_check.sh

# Should output:
# ✓ Production verification completed successfully!
# The framework is ready for production deployment.
```

### Step 2: Build Release Binary

```bash
# Build optimized release
cargo build --release -p foundry-cli

# Binary location:
# target/release/foundry
```

### Step 3: Build Docker Image

```bash
# Build image
docker build -t rustforge:latest .

# Test image locally
docker run -p 8000:8000 rustforge:latest

# Tag for registry
docker tag rustforge:latest your-registry/rustforge:v0.1.0
docker tag rustforge:latest your-registry/rustforge:latest

# Push to registry
docker push your-registry/rustforge:v0.1.0
docker push your-registry/rustforge:latest
```

### Step 4: Database Migration

```bash
# Backup database first!
pg_dump -U postgres myapp > backup_$(date +%Y%m%d).sql

# Run migrations
foundry migrate

# Verify migration
foundry migrate:status
```

### Step 5: Deploy Application

#### Option A: Docker Compose

```bash
# Start services
docker-compose up -d

# Check logs
docker-compose logs -f app

# Check health
curl http://localhost:8000/health
```

#### Option B: Kubernetes

```bash
# Apply manifests
kubectl apply -f k8s/

# Check deployment
kubectl get pods
kubectl logs -f deployment/rustforge

# Check health
kubectl port-forward deployment/rustforge 8000:8000
curl http://localhost:8000/health
```

#### Option C: Systemd Service

```bash
# Copy binary
sudo cp target/release/foundry /usr/local/bin/

# Create systemd service
sudo nano /etc/systemd/system/rustforge.service
```

```ini
[Unit]
Description=RustForge Application
After=network.target postgresql.service

[Service]
Type=simple
User=rustforge
WorkingDirectory=/opt/rustforge
ExecStart=/usr/local/bin/foundry serve
Restart=always
RestartSec=5

Environment="DATABASE_URL=postgres://user:pass@localhost/myapp"
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

```bash
# Start service
sudo systemctl daemon-reload
sudo systemctl enable rustforge
sudo systemctl start rustforge

# Check status
sudo systemctl status rustforge
```

### Step 6: Start Observability Stack

```bash
cd observability
docker-compose up -d

# Verify services
docker-compose ps

# Access dashboards
# Prometheus: http://localhost:9090
# Grafana: http://localhost:3000
# Jaeger: http://localhost:16686
```

### Step 7: Configure Monitoring & Alerts

```bash
# Import Grafana dashboards
# Login to Grafana (admin/admin)
# Import dashboard from observability/grafana-dashboards/

# Configure alerts in AlertManager
# Edit observability/alertmanager/alertmanager.yml

# Test alerts
curl -X POST http://localhost:9093/api/v1/alerts
```

### Step 8: Smoke Tests

```bash
# Health check
curl http://your-domain.com/health

# Should return: {"status": "ok"}

# Load test (optional)
ab -n 1000 -c 10 http://your-domain.com/

# Monitor metrics
curl http://your-domain.com/metrics
```

---

## Post-Deployment Checklist

### Immediate (Within 1 Hour)

- [ ] Application responding to requests
- [ ] Health check endpoint working
- [ ] Logs being collected
- [ ] Metrics being recorded
- [ ] No error spikes in logs
- [ ] Response times acceptable
- [ ] Database connections healthy

### Short-term (Within 24 Hours)

- [ ] Monitor error rates
- [ ] Check memory usage trends
- [ ] Verify backup schedules
- [ ] Test alert notifications
- [ ] Review access logs
- [ ] Monitor database performance
- [ ] Verify SSL certificates

### Medium-term (Within 1 Week)

- [ ] Analyze performance metrics
- [ ] Review capacity planning
- [ ] Test disaster recovery
- [ ] Update documentation
- [ ] Collect user feedback
- [ ] Review security logs
- [ ] Plan next release

---

## Rollback Procedure

If deployment fails:

### Step 1: Stop New Version

```bash
# Docker Compose
docker-compose down

# Kubernetes
kubectl rollout undo deployment/rustforge

# Systemd
sudo systemctl stop rustforge
```

### Step 2: Restore Database (if needed)

```bash
# Restore from backup
psql -U postgres myapp < backup_YYYYMMDD.sql
```

### Step 3: Start Previous Version

```bash
# Docker
docker-compose up -d

# Kubernetes
kubectl rollout undo deployment/rustforge

# Systemd
sudo systemctl start rustforge
```

### Step 4: Verify Rollback

```bash
# Check health
curl http://your-domain.com/health

# Check logs
docker-compose logs -f app
```

---

## Monitoring Checklist

### Key Metrics to Monitor

- [ ] Request rate (req/sec)
- [ ] Response time (p50, p95, p99)
- [ ] Error rate (%)
- [ ] CPU usage (%)
- [ ] Memory usage (MB)
- [ ] Database connections
- [ ] Cache hit rate (%)
- [ ] Active connections
- [ ] Queue depth

### Alerts to Configure

- [ ] Error rate > 1%
- [ ] Response time p99 > 1s
- [ ] Memory usage > 80%
- [ ] CPU usage > 80%
- [ ] Disk usage > 80%
- [ ] Database connection pool exhausted
- [ ] Certificate expiring within 30 days
- [ ] Service down > 1 minute

---

## Troubleshooting Guide

### Application Won't Start

1. Check logs: `docker-compose logs app`
2. Verify environment variables
3. Check database connectivity
4. Verify port availability: `netstat -tuln | grep 8000`

### High Memory Usage

1. Check for memory leaks
2. Review connection pooling settings
3. Analyze heap dumps
4. Check cache size limits

### Slow Response Times

1. Check database query performance
2. Review API endpoint logs
3. Check network latency
4. Verify caching is working
5. Review connection pool settings

### Database Connection Errors

1. Check database is running
2. Verify connection string
3. Check connection pool limits
4. Review firewall rules
5. Check database credentials

---

## Production Verification Command

Run this before and after deployment:

```bash
./scripts/production_check.sh
```

This script checks:
- ✅ Build status
- ✅ Test results
- ✅ Code quality
- ✅ Security audit
- ✅ Documentation
- ✅ Dependencies
- ✅ Configuration

---

## Maintenance Windows

### Regular Maintenance

- **Database backups**: Daily at 2 AM
- **Security updates**: Weekly
- **Dependency updates**: Monthly
- **Performance review**: Monthly
- **Disaster recovery test**: Quarterly

### Emergency Maintenance

Follow the incident response plan:

1. Assess impact
2. Notify stakeholders
3. Implement fix
4. Test thoroughly
5. Deploy
6. Post-mortem

---

## Support Contacts

- **DevOps Team**: devops@yourcompany.com
- **On-call**: oncall@yourcompany.com
- **Security**: security@yourcompany.com
- **PagerDuty**: https://yourcompany.pagerduty.com

---

## Additional Resources

- [Production Check Script](./scripts/production_check.sh)
- [CI/CD Report](./CI_CD_TESTING_REPORT.md)
- [Testing Guide](./TESTING.md)
- [Benchmark Guide](./BENCHMARKS.md)
- [Observability Setup](./observability/README.md)

---

**Last Updated:** November 5, 2025
**Version:** 1.0.0
