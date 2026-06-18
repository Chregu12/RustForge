# Welcome to RustForge

![RustForge Logo](https://img.shields.io/badge/RustForge-v1.0.0-blue)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

**RustForge** is a production-ready web application framework for Rust, inspired by Laravel. It provides a complete, batteries-included development experience targeting Laravel feature parity, including recent Laravel 13 additions such as `Cache::touch`, queue routing by job class, JSON:API resources, a provider-agnostic AI SDK (`rf-ai`), and vector/semantic search (`rf-vector`).

## What is RustForge?

RustForge brings the elegant developer experience of Laravel to the Rust ecosystem. Whether you're building APIs, web services, or full-stack applications, RustForge provides all the tools you need:

- **Elegant Syntax**: Write expressive, readable code
- **Complete Tooling**: 50+ CLI commands for rapid development
- **Battle-Tested**: Production-ready with comprehensive test coverage
- **Performant**: Built on Rust's zero-cost abstractions
- **Type-Safe**: Compile-time guarantees for reliability

## 🆕 Simplified Imports with `rf` Crate

RustForge now offers **ultra-simple imports** with the `rf` crate:

```rust
// Direct imports - Laravel style!
use rf::{Route, Auth, DB, Hash, Collection};

// Or use prelude for everything
use rf::prelude::*;

// 5 main modules for organization
use rf::web::*;        // HTTP, Views, API
use rf::data::*;       // DB, Cache, Validation
use rf::background::*; // Jobs, Events, Broadcast
use rf::services::*;   // Storage, Mail, Auth
use rf::helpers::*;    // Helper functions
```

**👉 [Learn more about Laravel Syntax](Laravel-Syntax)**

## 🆕 Phase 21: Complete Laravel Parity

RustForge now includes all major Laravel ecosystem packages:

| Package | Description |
|---------|-------------|
| **rf-dusk** | Browser testing with WebDriver/fantoccini |
| **rf-echo** | WebSocket broadcasting client (Pusher/Soketi) |
| **rf-envoy** | SSH deployment task runner |
| **rf-sail** | Docker development environment |
| **rf-spark** | SaaS billing with Stripe integration |

## 🆕 Laravel 13 Features

RustForge has adopted recent Laravel 13 capabilities:

| Feature | Package | Description |
|---------|---------|-------------|
| **Cache::touch** | `rf-cache` | Reset a cache entry's TTL without rewriting its value (`Cache::touch(key, ttl)`) |
| **Queue routing by class** | `rf-jobs` | Route jobs to queues by their type via `JobRouter::route::<Job>("queue")` |
| **JSON:API resources** | `rf-api-resources` | `jsonapi` module with `JsonApiResource`, `JsonApiDocument`, relationships |
| **AI SDK** | `rf-ai` | Provider-agnostic chat/embeddings/tool-calling agents with an Anthropic provider |
| **Vector search** | `rf-vector` | Embedding vectors, similarity metrics, in-memory store, and pgvector SQL helpers |

## 🆕 Additional Features

- **Blade Templates** - `@if`, `@foreach`, `@auth`, `@csrf` and more
- **Mailable Classes** - Structured emails with envelope, content, attachments
- **Notifications** - Multi-channel (Mail, Database, Slack)
- **Form Requests** - Automatic validation with rules
- **Exception Handler** - Global error handling
- **20+ Helper Macros** - `now!`, `bcrypt!`, `view!`, `redirect!`, etc.

## Quick Links

- **[Installation Guide](Installation)** - Get started with RustForge
- **[Quick Start](Quick-Start)** - Build your first application
- **[Laravel Syntax](Laravel-Syntax)** - 🆕 Use familiar Laravel syntax!
- **[Features](Features)** - Explore what RustForge offers
- **[API Documentation](API-Documentation)** - Detailed API reference
- **[Examples](Examples)** - Learn by example
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

## Key Features

### ORM & Database
- Eloquent-style ORM with relationships
- Query builder with type-safe queries
- Database migrations and seeders
- Multiple database support (PostgreSQL, MySQL, SQLite)

### Authentication & Security
- Built-in authentication system
- JWT and session-based auth
- Password hashing and validation
- CSRF protection
- Rate limiting

### HTTP & Routing
- Expressive routing system
- Middleware support
- Request validation
- Response formatting (JSON, XML, HTML)
- RESTful resource controllers

### Caching & Performance
- Multiple cache drivers (Redis, Memcached, File)
- Query result caching
- Cache tagging and expiration
- Performance optimizations

### Queue & Jobs
- Background job processing
- Multiple queue drivers
- Job retry and failure handling
- Scheduled tasks

### Templates & Views
- Blade-like templating (`@if`, `@foreach`, `@auth`)
- Template sections and stacks
- HTML escaping and raw output

### Mail & Notifications
- Mailable classes with structured emails
- Multi-channel notifications (Mail, DB, Slack)
- Markdown email support
- Queue support for async sending

### Additional Features
- Event system with listeners
- File storage (Local, S3)
- Broadcasting (WebSockets)
- GraphQL support
- Internationalization (i18n)
- Health checks and monitoring
- Audit logging

## Architecture

RustForge follows a modular architecture with clear separation of concerns:

```
rf-core/          # Core framework functionality
rf-orm/           # ORM and database layer
rf-web/           # HTTP routing and middleware
rf-auth/          # Authentication system
rf-validation/    # Input validation
rf-cache/         # Caching layer
rf-queue/         # Job queue system
rf-jobs/          # Background jobs
rf-mail/          # Email system
rf-storage/       # File storage
rf-broadcast/     # WebSocket broadcasting
rf-dusk/          # Browser testing (NEW)
rf-echo/          # Broadcasting client (NEW)
rf-envoy/         # SSH deployment (NEW)
rf-sail/          # Docker environment (NEW)
rf-spark/         # SaaS billing (NEW)
rf/               # Simplified imports (NEW)
rf-ai/            # Provider-agnostic AI SDK (NEW)
rf-vector/        # Vector & semantic search (NEW)
...               # 134+ total packages
```

## System Requirements

- **Rust**: 1.75 or higher
- **Database**: PostgreSQL 12+, MySQL 8+, or SQLite 3.35+
- **Cache** (optional): Redis 6+ or Memcached 1.6+
- **OS**: Linux, macOS, or Windows

## Community & Support

- **Repository**: https://github.com/Chregu12/RustForge
- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **License**: MIT OR Apache-2.0

## Getting Started

Ready to build something amazing? Start with our [Installation Guide](Installation) and then follow the [Quick Start](Quick-Start) tutorial to create your first RustForge application.

## Version Information

- **Current Version**: 1.0.0
- **Release Date**: December 20, 2024
- **Status**: Production Ready
- **Total Packages**: 134+
- **Rust Edition**: 2021

## Credits

RustForge is created and maintained by **chregu12** and inspired by the Laravel framework.

---

*Happy coding with RustForge!*
