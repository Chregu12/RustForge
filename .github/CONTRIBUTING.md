# Contributing to RustForge

First off, thank you for considering contributing to RustForge! It's people like you that make RustForge such a great framework.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](../CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible using our bug report template.

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

- A clear and descriptive title
- A detailed description of the proposed feature
- Laravel equivalent if applicable
- Example code showing the proposed API

### Pull Requests

1. Fork the repo and create your branch from `main`
2. If you've added code that should be tested, add tests
3. If you've changed APIs, update the documentation
4. Ensure the test suite passes
5. Make sure your code lints
6. Issue that pull request!

## Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/rustforge.git
cd rustforge

# Add upstream remote
git remote add upstream https://github.com/rustforge/rustforge.git

# Create a branch
git checkout -b feature/my-feature

# Install dependencies and run tests
cargo test

# Make your changes
# ...

# Run tests and linting
cargo test
cargo fmt
cargo clippy

# Commit and push
git add .
git commit -m "Add my feature"
git push origin feature/my-feature
```

## Coding Guidelines

### Rust Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address all warnings
- Use meaningful variable and function names
- Add doc comments to public APIs

### Testing

- Write unit tests for new functionality
- Write integration tests for features
- Aim for >80% code coverage
- Use descriptive test names

```rust
#[test]
fn test_user_can_be_created_with_valid_data() {
    // Test implementation
}
```

### Documentation

- Add rustdoc comments to all public APIs
- Include examples in documentation
- Update README.md if needed
- Add entries to CHANGELOG.md

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add user authentication
fix: resolve database connection pool issue
docs: update installation guide
test: add tests for query builder
refactor: simplify cache implementation
```

## Project Structure

```
rustforge/
├── crates/              # Framework crates
│   ├── rf-core/        # Core utilities
│   ├── rf-orm/         # ORM implementation
│   ├── rf-web/         # Web framework
│   └── ...
├── rustforge-starter/  # Application template
├── docs/               # Documentation
├── examples/           # Example applications
└── tests/              # Integration tests
```

## Adding a New Feature

1. **Discuss First**: Open an issue to discuss your feature
2. **Research Laravel**: Check how Laravel implements it
3. **Design API**: Propose the Rust API (maintain Laravel's elegance)
4. **Implement**: Write the code with tests
5. **Document**: Add rustdoc and user documentation
6. **Submit PR**: Create a pull request with all changes

## Laravel Compatibility

When implementing features:

1. Keep the API as close to Laravel as Rust allows
2. Document differences from Laravel
3. Provide migration examples for Laravel developers
4. Consider async implications

Example:

```rust
// Laravel: User::with('posts')->get()
// RustForge: User::with("posts").get().await?
```

## Review Process

1. A maintainer will review your PR
2. Address any feedback
3. Once approved, a maintainer will merge
4. Your contribution will be credited

## Recognition

Contributors are recognized in:
- CHANGELOG.md
- README.md (for significant contributions)
- Release notes

## Questions?

Feel free to:
- Open an issue for discussion
- Join our Discord server
- Email the maintainers

Thank you for contributing to RustForge! 🚀
