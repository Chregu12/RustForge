# My RustForge Application

[![App CI](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/app-ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/YOUR_REPO/actions/workflows/app-ci.yml)

A modern web application built with [RustForge](https://github.com/Chregu12/RustForge) - The Laravel experience for Rust.

## About

This is a fresh RustForge application. RustForge brings Laravel's elegant developer experience to the Rust ecosystem, providing you with a powerful, type-safe foundation for building web applications.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- PostgreSQL (or your preferred database)
- Redis (for caching and queues)

### Installation

1. Clone this repository
2. Copy the environment file:
   ```bash
   cp .env.example .env
   ```

3. Configure your database in `.env`

4. Run migrations:
   ```bash
   forge migrate
   ```

5. Start the development server:
   ```bash
   cargo run
   ```

6. Visit http://localhost:3000

## Project Structure

```
├── app/                # Application logic
│   ├── Http/          # Controllers & Middleware
│   ├── Models/        # Database models
│   └── Services/      # Business logic
├── config/            # Configuration files
├── database/          # Migrations, seeders, factories
├── routes/            # Route definitions
├── resources/         # Views, CSS, JavaScript
├── storage/           # File storage
└── tests/             # Test suite
```

## Available Commands

```bash
# Run the application
cargo run

# Run tests
cargo test

# Database migrations
forge migrate
forge migrate:rollback
forge migrate:fresh

# Code generation
forge make:model User
forge make:controller UserController
forge make:migration create_users_table

# Queue worker
forge queue:work

# Development
forge serve              # Start development server
forge tinker            # Interactive REPL
```

## CI/CD

This project comes with pre-configured GitHub Actions workflows:

### Continuous Integration (`.github/workflows/app-ci.yml`)
- **Runs on**: Every push and pull request to `main`, `master`, `develop`
- **Steps**:
  - Code formatting check (`cargo fmt`)
  - Linting with clippy
  - Build verification
  - Unit tests

### Docker Release (`.github/workflows/release-docker.yml`)
- **Runs on**: Version tags (`v*.*.*`)
- **Steps**:
  - Builds Docker image
  - Pushes to GitHub Container Registry (ghcr.io)
  - Creates GitHub Release

### Enable Database Tests
Uncomment the `db-tests` job in `app-ci.yml` to enable PostgreSQL integration tests.

### Deploying

1. Create a version tag:
   ```bash
   git tag v1.0.0
   git push origin v1.0.0
   ```

2. The workflow will:
   - Build and push Docker image to `ghcr.io/your-username/your-repo:1.0.0`
   - Create a GitHub Release with auto-generated notes

## Learning RustForge

- [Documentation](https://github.com/Chregu12/RustForge/wiki)
- [Quick Start Guide](https://github.com/Chregu12/RustForge/wiki/Quick-Start)
- [API Reference](https://github.com/Chregu12/RustForge/wiki/API-Documentation)

## Contributing

Thank you for considering contributing to this project!

## License

This project is open-sourced software licensed under the [MIT license](LICENSE).
