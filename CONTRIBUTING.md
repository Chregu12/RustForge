# Contributing to RustForge

We appreciate your interest in contributing to RustForge! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md) to ensure a welcoming and inclusive community.

## Getting Started

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
- **Git** - For cloning and version control
- **A database** - SQLite (recommended for dev), PostgreSQL, or MySQL

### Local Development Setup

1. **Fork the repository**
   ```bash
   git clone https://github.com/YOUR_USERNAME/RustForge.git
   cd RustForge
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

3. **Set up environment**
   ```bash
   cp .env.example .env
   # Edit .env with your local database configuration
   ```

4. **Build and test**
   ```bash
   cargo build
   cargo test
   cargo check
   ```

5. **Run the project**
   ```bash
   cargo run
   ```

## Development Workflow

### Code Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code before committing
- Run `cargo clippy` to catch common mistakes

```bash
cargo fmt
cargo clippy -- -D warnings
```

### Commit Messages

Please use clear, descriptive commit messages:

```
feat: Add new tinker command for database REPL
fix: Resolve database connection pooling issue
docs: Update installation instructions
refactor: Improve CLI command structure
test: Add tests for CRUD operations
```

### Pull Request Process

1. **Create descriptive PR** with:
   - Clear title and description
   - Reference to any related issues (#123)
   - Description of changes and motivation

2. **Code review checklist:**
   - [ ] Code follows style guidelines
   - [ ] Tests added/updated
   - [ ] Documentation updated
   - [ ] No breaking changes (or documented)

3. **Ensure CI passes:**
   - All tests pass: `cargo test`
   - No compiler warnings: `cargo clippy`
   - Code formatted: `cargo fmt --check`

## Reporting Bugs

Report bugs by creating a GitHub Issue with:

1. **Clear title** describing the issue
2. **Rust version:** `rustc --version`
3. **OS and system info**
4. **Steps to reproduce**
5. **Expected behavior**
6. **Actual behavior**
7. **Error message/backtrace**

## Suggesting Features

Feature suggestions are welcome! Please:

1. Check existing issues/PRs first
2. Use clear, descriptive title
3. Explain the use case and benefit
4. Provide examples if applicable

## Project Structure

```
RustForge/
├── crates/
│   ├── foundry-application/  # Application layer (commands)
│   ├── foundry-cli/          # CLI interface
│   ├── foundry-domain/       # Core domain models
│   ├── foundry-infra/        # Infrastructure (DB, cache)
│   ├── foundry-plugins/      # Plugin interfaces
│   ├── foundry-api/          # API layer
│   └── foundry-storage/      # File storage
├── migrations/               # Database migrations
├── seeds/                    # Database seeders
├── tests/                    # Integration tests
└── README.md                 # This file
```

## Key Components to Know

### Commands Layer (`foundry-application/src/commands/`)
- Each command implements `FoundryCommand` trait
- CommandDescriptor defines metadata
- Execute method handles the logic

### Tinker REPL (`tinker.rs`)
- Interactive REPL for database operations
- Query parsing and execution
- Session management

### Database (`foundry-infra`)
- Sea-ORM integration
- Connection pooling
- Migration/seeding logic

## Testing Guidelines

- Write tests for new features
- Run full test suite before submitting PR: `cargo test`
- Use integration tests for complex features

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Documentation

- Update README.md for user-facing changes
- Add inline code comments for complex logic
- Update API docs for public interfaces
- Use `///` doc comments on public items

## Performance Considerations

- Use async/await properly with Tokio
- Avoid blocking operations on async threads
- Be mindful of database query performance
- Profile before optimizing

## Security

- Never commit credentials or secrets
- Use `.env.example` for configuration templates
- Follow Rust security best practices
- Report security issues privately to maintainers

## Questions?

- **GitHub Issues** - For bugs and features
- **GitHub Discussions** - For questions and ideas
- **Documentation** - Check existing docs first

---

Thank you for contributing to RustForge! Your effort helps make this project better for everyone. 🚀

**Happy coding!**
