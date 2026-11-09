# Installation

<!-- rf-status:begin -->
Workspace version: **1.0.0-rc.3** (development snapshot). Minimum supported Rust: **1.96.0**. RustForge is consumed from Git; no crates.io release is required.
<!-- rf-status:end -->

RustForge is not published to crates.io. Use the pinned Git implementation
below. It includes the APIs shown in Quick Start; older RC tags do not.

## Rust toolchain

The supported minimum and the repository toolchain are both **Rust 1.96.0**.
The earlier 1.79 claim was not verified: the repository override caused its CI
job to compile with 1.96. The corrected job explicitly invokes
`cargo +1.96.0 check --locked -p rf -p rustforge`.

```sh
rustup toolchain install 1.96.0 --profile minimal
rustup default 1.96.0
rustc +1.96.0 --version
```

## Dependencies

<!-- rf-dependencies:begin -->
```toml
[dependencies]
rf = { git = "https://github.com/Chregu12/RustForge", rev = "07b84ba3a61669ec64ebc9444d94166fc8eea13d" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
<!-- rf-dependencies:end -->

The explicit Axum, Tokio and Serde dependencies are needed by the application
code and generated models. `rf` contains the core; opt into extensions only when
needed. When developing inside a clone, replace the `rf` dependency with a path
to `crates/rf`.

## Verification

Follow [Quick Start](Quick-Start). Its SQLite example creates its own schema.
For PostgreSQL, configure a `DatabaseManager` and inject it with
`App::new().database(db)`; use PostgreSQL migrations, including a generated `id`.
Do not send the SQLite demo DDL to PostgreSQL.

The compiler version in `rust-toolchain.toml` overrides `rustup default` when
working inside this repository. Use an explicit `cargo +VERSION` when validating
a particular compiler version.
