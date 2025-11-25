# Welcome to RustForge

![RustForge Logo](https://img.shields.io/badge/RustForge-v1.0.0-blue)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

**RustForge** is a production-ready web application framework for Rust, inspired by Laravel. It provides a complete, batteries-included development experience with 100% Laravel 12 feature parity.

## What is RustForge?

RustForge brings the elegant developer experience of Laravel to the Rust ecosystem. Whether you're building APIs, web services, or full-stack applications, RustForge provides all the tools you need:

- **Elegant Syntax**: Write expressive, readable code
- **Complete Tooling**: 50+ CLI commands for rapid development
- **Battle-Tested**: Production-ready with comprehensive test coverage
- **Performant**: Built on Rust's zero-cost abstractions
- **Type-Safe**: Compile-time guarantees for reliability

## 🆕 New: Ultimate Laravel Experience

RustForge now supports **Laravel-identical syntax** with the `rustforge!` block - write Rust exactly like Laravel PHP:

```rust
// That's it! No imports needed!
rustforge! {
    Model!(User: name, email, hidden password);

    async fn index() -> Response {
        let users = User::where("active", true)
            .orderBy("name", "asc")
            .get();  // No .await needed!
        Response::json(users)
    }

    async fn store(data: Json<Value>) -> Response {
        let user = User::create(data.0);
        Response::json(user).status(201)
    }
}
```

**✨ What's automatic:**
- ✅ `use rustforge::*;` - no manual imports
- ✅ `#[auto_await]` applied to all async functions
- ✅ `.await` added to async calls automatically
- ✅ `where` keyword works like Laravel

**👉 [Learn more about Laravel Syntax](Laravel-Syntax)**

## 🆕 New Features in Latest Version

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
rf-http/          # HTTP routing and middleware
rf-auth/          # Authentication system
rf-validation/    # Input validation
rf-cache/         # Caching layer
rf-queue/         # Job queue system
rf-jobs/          # Background jobs
rf-mail/          # Email system
rf-storage/       # File storage
rf-broadcast/     # WebSocket broadcasting
...               # 85+ total packages
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
- **Release Date**: November 23, 2025
- **Status**: Production Ready
- **Rust Edition**: 2021

## Credits

RustForge is created and maintained by **chregu12** and inspired by the Laravel framework.

---

*Happy coding with RustForge!*
