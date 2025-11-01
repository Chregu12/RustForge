# Changelog

All notable changes to RustForge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Interactive Tinker REPL for database operations
- Full CRUD support in Tinker (find, list, create, update, delete)
- Raw SQL query execution in Tinker
- Multi-database support (SQLite, PostgreSQL, MySQL)
- CLI command scaffolding system
- Database migrations and seeding
- Event system and background jobs
- Request validation framework
- Middleware support

### Changed
- Project renamed from Foundry to RustForge for broader scope

### Fixed
- SQL injection protection in Tinker commands
- Database connection pooling improvements

## [0.1.0] - 2025-10-31

### Added
- Initial release of RustForge Framework
- Core CLI application framework
- Basic command structure and registry
- Database abstraction layer
- REPL foundation for future development

### Features
- 22+ CLI commands for development
- 6 modular crates for different concerns
- Async/await support with Tokio
- Sea-ORM integration
- Event-driven architecture foundation

---

## Future Releases

### Planned for v0.2.0
- [ ] Authentication & Authorization (Sessions, JWT)
- [ ] Real-Time Features (WebSockets)
- [ ] GraphQL Support
- [ ] Enhanced Tinker with model introspection
- [ ] Custom Tinker commands

### Planned for v0.3.0
- [ ] Admin Dashboard
- [ ] Package Manager (package-like system)
- [ ] Testing Framework enhancements
- [ ] API Documentation Auto-Generation
- [ ] Performance Monitoring

---

## How to Report Issues

Please report bugs and request features on our [GitHub Issues](https://github.com/Chregu12/RustForge/issues) page.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
