# Quick Start Guide

Get up and running with RustForge in 5 minutes! 🚀

## Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
- **Git** - For cloning the repository

## 1. Clone the Repository

```bash
git clone https://github.com/Chregu12/RustForge.git
cd RustForge
```

## 2. Set Up Environment

```bash
cp .env.example .env
```

The default `.env` is configured for SQLite which requires no additional setup.

For other databases, edit `.env`:

```bash
# For PostgreSQL
DATABASE_URL=postgresql://username:password@localhost:5432/rustforge_dev

# For MySQL
DATABASE_URL=mysql://username:password@localhost:3306/rustforge_dev
```

## 3. Build the Project

```bash
cargo build
```

First build may take 2-3 minutes as dependencies are compiled.

## 4. Run Tests

```bash
cargo test
```

## 5. Start Using RustForge

### Generate Your First Model

```bash
cargo run -- make:model User -mcs
```

This generates:
- Model definition
- Database migration
- Controller
- Seeder

### Run Migrations

```bash
cargo run -- migrate
```

### Use the Interactive Tinker REPL

```bash
cargo run -- tinker
```

Then in the Tinker prompt:

```
tinker> list users
tinker> create users {"name": "John Doe", "email": "john@example.com"}
tinker> find users 1
tinker> update users 1 {"name": "Jane Doe"}
tinker> delete users 1
tinker> exit
```

### List All Available Commands

```bash
cargo run -- list
```

## 6. Production Build

```bash
cargo build --release
```

Binary will be at `./target/release/foundry-cli`

## Next Steps

- 📖 **Read the full [README.md](README.md)** for comprehensive documentation
- 🤝 **Check [CONTRIBUTING.md](CONTRIBUTING.md)** to contribute
- 📋 **Review [CHANGELOG.md](CHANGELOG.md)** for latest features
- 🔒 **See [SECURITY.md](SECURITY.md)** for security best practices

## Common Commands

```bash
# Code generation
cargo run -- make:migration create_posts_table
cargo run -- make:controller Api/PostController --api
cargo run -- make:job ProcessEmail --async

# Database
cargo run -- migrate
cargo run -- migrate:fresh --seed
cargo run -- db:seed

# Development
cargo run -- serve                  # Start dev server
cargo run -- list                   # List all commands
cargo run -- test                   # Run tests

# Interactive
cargo run -- tinker                 # Start REPL console
```

## Troubleshooting

### "Rust not found"
Install Rust from [rustup.rs](https://rustup.rs):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### "DATABASE_URL not found"
Make sure `.env` file exists and has a valid DATABASE_URL:
```bash
cp .env.example .env
```

### "Compilation errors"
Update your Rust toolchain:
```bash
rustup update
cargo clean
cargo build
```

### Database connection issues
- For SQLite: No setup needed, auto-created
- For PostgreSQL: Ensure PostgreSQL is running and credentials are correct
- For MySQL: Ensure MySQL is running and credentials are correct

## Getting Help

- 💬 **GitHub Issues** - Report bugs or request features
- 📚 **Documentation** - Read the full README.md
- 🤝 **Contributing** - See CONTRIBUTING.md

---

**Happy coding with RustForge!** ⚡

For detailed information, see the [README.md](README.md)
