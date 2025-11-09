# Changelog

All notable changes to the RustForge Framework will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - Phase 2: Modular Architecture Rebuild

#### PR-Slice #7: Background Jobs & Queue System (2025-11-09)

**rf-jobs v0.1.0**
- **Job Trait**: Async job execution with retry logic
  - Customizable queue names
  - Configurable max attempts (default: 3)
  - Exponential backoff between retries
  - Timeout support per job
  - Failed job callback
- **JobContext**: Rich execution context
  - Job ID, queue name, attempt number
  - Timestamps (dispatched_at, started_at)
  - Logging helpers (log, warn, error)
  - Final attempt detection
- **Queue Manager**: Redis-backed job queue
  - dispatch() - Immediate job dispatch
  - dispatch_later() - Delayed job execution
  - pop() / pop_nowait() - Job retrieval
  - Failed job queue (Dead Letter Queue)
  - Queue size and clear operations
  - Retry failed jobs
- **Worker Pool**: Concurrent job processing
  - Configurable worker count (default: CPU cores)
  - Multi-queue support with priority
  - Job timeout enforcement
  - Graceful shutdown with job completion
  - Auto-retry with backoff
- **Scheduler**: Cron-like job scheduling
  - 6-field cron expressions (sec min hour day month dow)
  - Register scheduled jobs
  - Background execution
  - Graceful shutdown
- **Error Handling**: Comprehensive error types
  - JobError (ExecutionFailed, Timeout, Custom)
  - QueueError (ConnectionError, JobNotFound)
  - WorkerError, SchedulerError
  - Automatic conversion to AppError
- **Testing**: 11 unit tests, all passing
- **Code**: 1,400 production lines, 138 test lines, 285 comment lines

**examples/jobs-demo**
- **5 Job Types**: Email, Image, Report, Cleanup, Failing
- **Worker Pool Demo**: 2 concurrent workers, 4 queues
- **Scheduler Demo**: Cron-based cache cleanup
- **Delayed Jobs**: 10-second delay demonstration
- **Retry Logic**: Failing job with 2 retries
- **Queue Status**: Real-time queue monitoring
- **Graceful Shutdown**: Ctrl+C handling
- **Code**: 340 lines + comprehensive inline docs

**API Documentation**
- API Sketch: 06-rf-jobs-background-queue.md (1,200+ lines)
  - Job trait reference
  - Queue manager API
  - Worker pool configuration
  - Scheduler cron patterns
  - Job chaining (future)
  - Job batching (future)
  - Monitoring strategies

#### PR-Slice #6: Validation & Forms (2025-11-09)

**rf-validation v0.1.0**
- **ValidatedJson Extractor**: Automatic request validation in Axum
  - `FromRequest` impl for type-safe validation
  - Validates after JSON parsing, before handler
  - Returns 422 Unprocessable Entity on failure
  - Zero boilerplate for common cases
- **30+ Validation Rules**: Built on validator crate v0.18
  - String: email, url, length, contains, regex
  - Numeric: range, custom
  - Collections: length
  - Custom: user-defined validation functions
  - Nested: validate nested structs and collections
- **ValidationErrors**: Field-level error tracking
  - Groups errors by field name
  - Stores error code, message, and params
  - Serializes to JSON (RFC 7807 compatible)
  - Converts from validator::ValidationErrors
- **FieldError**: Individual validation error
  - Error code (e.g., "email", "length")
  - Human-readable message
  - Optional parameters (min, max values)
- **RFC 7807 Error Responses**: Standard error format
  - Type, title, status fields
  - Detailed error messages per field
  - Parameter information for debugging
- **Error Conversions**: Seamless integration
  - `validator::ValidationErrors` → `ValidationErrors`
  - `ValidationErrors` → `AppError` (rf-core)
  - Preserves all error details
- **Testing**: 8 unit tests + 4 integration tests = 12/12 passing (100%)
- **Code**: 647 production lines, 154 test lines, 136 doc lines

**examples/validation-demo**
- **8 Validation Scenarios**: Comprehensive demonstration
  - Basic validation (email, length, range)
  - URL validation (with optional fields)
  - Custom regex (SKU pattern: ABC-1234)
  - Contains validation (must contain "@")
  - Custom validation function (username rules)
  - Nested validation (Address in Order)
  - Multiple rules per field (title validation)
  - Optional field validation (profile updates)
- **8 HTTP Endpoints**: Full REST API example
  - POST /users: Create user with validation
  - POST /websites: Validate URLs
  - POST /products: Regex validation
  - POST /search: Contains validation
  - POST /register: Custom validators
  - POST /orders: Nested validation
  - POST /blog-posts: Multiple rules
  - POST /profile: Optional fields
- **Integration Tests**: 4 tests validating HTTP responses
  - Valid data returns 201 Created
  - Invalid email returns 422 with field errors
  - Short password returns 422 with params
  - Age out of range returns 422
- **Code**: 450 lines + 86 doc lines

**API Documentation**
- API Sketch: 05-rf-validation-forms.md (950+ lines)
  - 30+ validation rule reference
  - ValidatedJson usage patterns
  - Custom validation functions
  - Error handling strategies
  - Testing best practices
  - Performance considerations
  - Security considerations (DoS, injection)

#### PR-Slice #5: Authentication & Security (2025-01-09)

**rf-auth v0.1.0**
- **PasswordHasher**: Secure password hashing
  - Bcrypt support (cost 4-31, default 12)
  - Argon2 support for modern hashing
  - Auto-detection of hash format
  - Timing-safe comparison methods
  - ~250ms per hash at cost 12 (secure default)
- **JwtManager**: JSON Web Token generation and validation
  - Access token generation (customizable expiry)
  - Refresh token generation (7 days default)
  - Token validation with expiration checks
  - HS256 algorithm (HMAC with SHA-256)
  - Secret key minimum 32 characters
- **Claims Structure**: Standard + custom JWT claims
  - Standard: sub, exp, iat, jti
  - Custom: user_id, roles
  - Role helpers: has_role(), has_any_role(), has_all_roles()
- **Authentication Middleware**: Axum integration
  - auth_layer: JWT validation middleware
  - Extracts token from Authorization header
  - Adds claims to request extensions
  - Returns 401 Unauthorized on invalid tokens
- **Role-Based Access Control**: require_role() helper
  - Checks if user has required role
  - Returns 403 Forbidden if role missing
- **Error Handling**: Comprehensive AuthError types
  - InvalidCredentials, TokenExpired, InvalidToken
  - WeakPassword, HashingFailed, JwtError
  - UserNotFound, EmailExists
  - Automatic conversion to AppError (rf-core)
- **Testing**: 15 unit tests = 15/15 passing (100%)
- **Code**: 960 production lines, 240 test lines, 850 doc lines

**examples/auth-demo**
- **Complete Authentication API**: 7 endpoints
  - POST /register: User registration with password hashing
  - POST /login: Login with JWT token generation
  - POST /refresh: Token refresh mechanism
  - GET /profile: Protected route (requires auth)
  - GET /admin: Admin route (requires auth + admin role)
  - GET /health: Health check
  - GET /: API documentation
- **Features Demonstrated**:
  - Bcrypt password hashing (cost 12)
  - JWT token generation (24-hour expiry)
  - Refresh tokens (7-day expiry)
  - Role-based access control
  - Request/Response DTOs with serde
  - Comprehensive logging
- **Code**: 350 lines + extensive inline docs

**API Documentation**
- API Sketch: 04-rf-auth-authentication-security.md (850+ lines)
  - Password hashing patterns and best practices
  - JWT token lifecycle and claims structure
  - Authentication middleware integration
  - User registration and login flows
  - Token refresh mechanism
  - Password reset flow
  - Security best practices (CSRF, rate limiting)
  - Configuration structure
  - Testing strategies
  - Performance considerations

#### PR-Slice #4: ORM & Database Integration (2025-01-09)

**rf-orm v0.1.0**
- **DatabaseManager**: Connection pooling and management
  - Multi-database support: SQLite, PostgreSQL, MySQL via SeaORM
  - Configurable pool size, timeouts, and connection limits
  - Health check via ping() method
  - Graceful shutdown with connection draining
  - Password masking in logs for security
- **DatabaseConfig**: Type-safe database configuration
  - URL, max/min connections, timeouts
  - Serde serialization with Duration helpers
  - Integration with rf-config
- **DbError Types**: Comprehensive error handling
  - 8 error variants (NotFound, UniqueViolation, etc.)
  - Automatic conversion to rf-core AppError
  - Detailed error messages with context
- **SoftDelete Trait**: Optional soft deletion support
  - soft_delete() sets deleted_at timestamp
  - restore() clears deleted_at
  - is_deleted() detection
  - Preserves data for audit trails
- **Testing Utilities**: TestDatabase for easy testing
  - In-memory SQLite database
  - Automatic cleanup after tests
  - Connection access for queries
- **Testing**: 20 unit tests = 20/20 passing (100%)
- **Code**: 890 production lines, 300 test lines, 200 doc lines

**examples/database-demo**
- **Complete CRUD Demonstration**: 16-step comprehensive demo
  - Database connection and setup
  - Entity definition with SeaORM
  - Insert, update, delete operations
  - Query filtering and ordering
  - Soft delete and restore
  - Count and aggregate operations
- **User Entity**: Full example with soft delete
  - Primary key, unique constraints
  - Timestamps (created_at, updated_at, deleted_at)
  - SoftDelete trait implementation
- **Output**: Beautiful step-by-step logging with emojis
- **Code**: 305 lines + 400 doc lines
- **README**: Comprehensive guide with examples

**API Documentation**
- API Sketch: 03-rf-orm-database-integration.md (700+ lines)
  - DatabaseManager API
  - Entity definition patterns
  - Query builder examples
  - Transaction support
  - Migration patterns
  - Testing strategies
  - Performance considerations
  - Security best practices

#### PR-Slice #3: Configuration & Dependency Injection (2025-01-09)

**rf-config v0.1.0**
- **Hierarchical Configuration Loading**: Type-safe config with three-tier precedence
  - Default values (hardcoded in types)
  - Environment-specific files (`config/{env}.toml`)
  - Environment variables (`APP__SECTION__KEY`)
- **Type-Safe Structures**: AppConfig, ServerConfig, DatabaseConfig, AuthConfig
- **Validation**: Fail-fast validation on startup
  - Non-zero port, workers, connections
  - Production secrets must not be defaults
- **Testing**: 11 unit tests + 2 doc tests = 13/13 passing (100%)
- **Code**: 359 production lines, 168 test lines, 95 doc lines

**rf-container v0.1.0**
- **Dependency Injection Container**: Type-safe service registry
  - Three lifecycle scopes: Singleton, Scoped (future), Transient
  - Factory-based registration with `Arc<T>` instances
  - Type-erased storage via `TypeId` and `Any`
  - Thread-safe via `Arc<Mutex<_>>`
- **API Methods**: register(), resolve(), has(), remove(), clear()
- **Testing**: 14 unit tests + 10 doc tests = 24/24 passing (100%)
- **Code**: 450 production lines, 180 test lines, 140 doc lines

**examples/hello**
- **Minimal Integration Demo**: Complete Phase 2 framework showcase
  - Kubernetes-ready endpoints: /health, /ready, /metrics
  - Application endpoints: /, /echo
  - Full middleware stack: RequestID, Tracing, Timeout, CORS, Compression
  - RFC 7807 error responses
  - Environment-based configuration
  - Dependency injection integration
- **Code**: 280 lines + 230 doc lines
- **README**: Complete setup and usage guide

#### PR-Slice #2: rf-web - Axum Integration (2025-01-09)

**rf-web v0.1.0**
- **RouterBuilder**: Ergonomic router configuration
  - 5 middleware modules: RequestId, Tracing, CORS, Timeout, Compression
  - Configurable middleware stack
  - Method chaining API
- **Testing**: 16/16 tests passing (1 ignored)
- **Code**: 600+ production lines

#### PR-Slice #1: rf-core - Error Handling (2025-01-09)

**rf-core v0.3.0**
- **RFC 7807 Problem Details**: Standard error responses for HTTP APIs
  - `AppError` enum with 9 error variants mapping to HTTP status codes
  - `ProblemDetails` struct with full RFC 7807 compliance
  - `AppResult<T>` type alias for ergonomic error handling
- **Request Context**: Trace IDs for log correlation
  - `RequestContext` with unique UUID v4 trace IDs per request
  - Environment detection (Development, Staging, Production)
  - Auto-detection via `APP_ENV` environment variable
  - Helper methods: `is_development()`, `is_production()`, `is_staging()`
- **Environment-Aware Error Handling**:
  - Development mode: Shows full backtraces and error details
  - Production mode: Hides sensitive information, shows generic messages
- **Optional Validation Support**: Feature-gated `validator` integration
- **Comprehensive Testing**: 24 unit tests + 13 doc tests, 100% passing
- **Full Documentation**: API docs, examples, README, and 7 ADRs

#### Architecture Documentation
- ADR-001: Web Framework Choice (Axum + Tower)
- ADR-002: Error Handling (RFC 7807 Problem Details)
- ADR-003: Dependency Injection (Service Registry + Scopes)
- ADR-004: Observability & Tracing (OpenTelemetry)
- ADR-005: Configuration Management (config + dotenvy)
- ADR-006: ORM Choice (SeaORM)
- ADR-007: Job Queue Backend (Redis + Postgres)
- API Sketch: 01-rf-core-error-handling.md (400+ lines)
- API Sketch: 02-rf-web-axum-integration.md (300+ lines)
- API Sketch: 03-rf-orm-database-integration.md (700+ lines)
- Taskboard: 23 stories across 10 scopes with DoD
- PR-Slice Summary: PR-SLICE-03.md (650+ lines)
- PR-Slice Summary: PR-SLICE-04.md (800+ lines)

### Phase 2 Statistics (PR-Slices #1-7)
- **Total Production Code**: 5,800+ lines
- **Total Test Code**: 1,380+ lines
- **Total Documentation**: 6,700+ lines
- **Files Created**: 50 new files
- **Tests**: 168/168 passing (100%)
- **Test Coverage**: ~95%
- **Clippy Warnings**: 0 (production code)
- **Crates Created**: 8 (rf-core, rf-web, rf-config, rf-container, rf-orm, rf-auth, rf-validation, rf-jobs)
- **Examples**: 5 (hello, database-demo, auth-demo, validation-demo, jobs-demo)

---

## [0.2.0] - 2025-11-08

### Added - Production Features

#### Queue System (foundry-queue)
- Redis-backed job queue with persistence
- Priority queues and delayed jobs
- Retry logic with exponential backoff
- Dead Letter Queue (DLQ) for failed jobs
- Worker pool with graceful shutdown
- **Code**: 2,500+ lines production code

#### Cache System (foundry-cache)
- Redis-backed caching with TTL support
- Type-safe cache operations via serde
- Connection pooling with deadpool-redis
- Cache tags for bulk invalidation
- **Code**: 800+ lines production code

#### Validation System (foundry-forms)
- 27+ validation rules (Laravel parity)
- Custom rule support via closures
- FormRequest pattern with auto-validation
- Field-level error messages (i18n-ready)
- **Code**: 1,200+ lines production code
- **Tests**: 90+ test cases

#### Security Features
- **CSRF Protection**: Token generation and validation
- **Rate Limiting**: Fixed window and sliding window algorithms
- **Authorization**: Gates & Policies pattern
- **OAuth**: State validation with proper expiration (CRITICAL FIX)
- **Code**: 1,500+ lines production code

### Fixed
- **CRITICAL**: OAuth state expiration security bug (u64→u128 milliseconds precision)
- 20+ compilation errors across test suite
- Borrow checker errors in authorization tests (added Copy trait)
- Missing imports in scaffolding module

### Changed
- Documentation: Changed from 70% to honest 50-53% Laravel parity
- Added "NOT PRODUCTION READY" warnings to documentation
- Updated README with realistic feature status

### Documentation
- SENIOR_ARCHITECT_REVIEW.md: Grade 6.5/10 with 5 critical problems
- TEAM_COORDINATION.md: 5-week development plan (1,000+ lines)
- VALIDATION_GUIDE.md: Complete validation reference (650+ lines)
- SECURITY_AUDIT.md: OWASP security audit
- TEST_REPORT.md: Compilation errors and coverage analysis (400+ lines)

### Statistics
- **New Code**: ~8,000 lines production code
- **New Tests**: ~2,000 lines test code
- **Documentation**: ~10,000 lines
- **New Crates**: 4 (queue, cache, forms, oauth)
- **Test Suite**: Improved from 20% to 90% compilation success

---

## [0.1.0] - 2025-11-07

### Initial Release

#### Core Features
- CLI framework with Artisan-like command system
- Application structure (foundry-application)
- Domain layer (foundry-domain)
- Infrastructure layer (foundry-infra)
- Plugin system (foundry-plugins)
- Storage abstraction (foundry-storage)
- GraphQL support (foundry-graphql)
- Testing utilities (foundry-testing)

#### TIER 1 Features
- Interactive prompts (foundry-interactive)
- Console commands (foundry-console)
- Mail system (foundry-mail)
- Scheduling (foundry-scheduling)
- Notifications (foundry-notifications)
- Multi-tenancy (foundry-tenancy)

#### TIER 2 Features
- Service container (foundry-service-container)
- API resources (foundry-resources)
- Soft deletes (foundry-soft-deletes)
- Audit logging (foundry-audit)
- Full-text search (foundry-search)
- OAuth client (foundry-oauth)
- OAuth server (foundry-oauth-server)
- Auth scaffolding (foundry-auth-scaffolding)
- Configuration (foundry-config)
- Rate limiting (foundry-ratelimit)
- Internationalization (foundry-i18n)
- Broadcasting (foundry-broadcast)

#### TIER 3 Features
- Admin panel (foundry-admin)
- Data export (foundry-export)
- Form builder (foundry-forms)
- HTTP client (foundry-http-client)
- Enhanced REPL (foundry-tinker-enhanced)
- Maintenance mode (foundry-maintenance)
- Health checks (foundry-health)
- Environment management (foundry-env)
- Asset compilation (foundry-assets)

#### Infrastructure
- Command executor (foundry-command-executor)
- Signal handler (foundry-signal-handler)
- Observability (foundry-observability)

### Statistics
- **Total Crates**: 40+
- **Lines of Code**: ~50,000+
- **Commands**: 30+
- **Laravel Parity**: ~50%

---

## [0.0.1] - 2025-11-06

### Initial Prototype
- Basic CLI structure
- Command registration system
- Simple application framework

---

[Unreleased]: https://github.com/Chregu12/RustForge/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/Chregu12/RustForge/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Chregu12/RustForge/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/Chregu12/RustForge/releases/tag/v0.0.1
