# Ignored Tests Analysis Report

**Generated:** Sa. 15 Nov. 2025 12:09:24 CET
**Total Ignored Tests:** 101

## Summary

| Category | Count |
|----------|-------|
| Database Tests | 14 |
| Redis Tests | 61 |
| S3/AWS Tests | 1 |
| Integration Tests | 3 |
| Other | 22 |
| **Total** | **101** |

## Progress

- **Target:** 0 ignored tests (100% enabled)
- **Current:** 101 ignored tests
- **Progress:** -13% complete

## Action Plan

### Phase 1: Database Tests (14 tests)

1. Start PostgreSQL test database
2. Run migrations
3. Enable tests incrementally
4. Fix failures

### Phase 2: Redis Tests (61 tests)

1. Start Redis test server
2. Enable cache tests
3. Enable queue tests

### Phase 3: S3 Tests (1 tests)

1. Start MinIO server
2. Configure test credentials
3. Enable storage tests

### Phase 4: Integration Tests (3 tests)

1. Set up complete test infrastructure
2. Enable end-to-end tests
3. Verify all components work together

## Detailed Breakdown

### By Crate

- `rf-orm`: 13 tests
- `foundry-api`: 3 tests
- `foundry-cache`: 2 tests
- `rf-storage`: 2 tests
- `rf-queue`: 16 tests
- `rf-eloquent`: 1 tests
- `rf-jobs`: 16 tests
- `rf-web`: 1 tests
- `rf-ratelimit`: 4 tests
- `foundry-queue`: 4 tests
- `rf-broadcast`: 4 tests
- `foundry-oauth-server`: 1 tests
- `rf-cache`: 19 tests
- `rf-broadcasting`: 2 tests

## Commands

```bash
# Start test infrastructure
docker-compose -f tests/docker-compose.test.yml up -d

# Run ignored tests
cargo test -- --ignored

# Run specific category
cargo test --test '*relationship*' -- --ignored

# Check infrastructure health
docker-compose -f tests/docker-compose.test.yml ps
```

## Next Steps

1. [ ] Set up Docker Compose test infrastructure
2. [ ] Enable database tests (highest priority)
3. [ ] Enable Redis tests
4. [ ] Enable S3 tests
5. [ ] Enable integration tests
6. [ ] Achieve 100% test enablement

---

**Target Date:** 2-4 weeks
**Owner:** QA Team
