# My RustForge Application

A modern web application built with [RustForge](https://github.com/rustforge/rustforge) - The Laravel experience for Rust.

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

## Learning RustForge

- [Documentation](https://rustforge.dev/docs)
- [Quick Start Guide](https://rustforge.dev/docs/quickstart)
- [API Reference](https://docs.rs/rustforge)

## Contributing

Thank you for considering contributing to this project!

## License

This project is open-sourced software licensed under the [MIT license](LICENSE).
