# RustForge — Release Engineering Guide

> **Monorepo policy:** RustForge is NOT published to crates.io. All releases are
> git tags on the `main` branch. Downstreams consume the framework as a git or
> path dependency (see [Downstream consumption](#downstream-consumption) below).

---

## Table of contents

1. [SemVer policy](#semver-policy)
2. [Stable supported surface](#stable-supported-surface)
3. [Experimental surface (not covered by 1.0 guarantee)](#experimental-surface)
4. [MSRV policy](#msrv-policy)
5. [Deprecation policy](#deprecation-policy)
6. [Downstream consumption](#downstream-consumption)
7. [Release procedure](#release-procedure)
8. [Security](#security)

---

## SemVer policy

RustForge follows [Semantic Versioning 2.0.0](https://semver.org/).

| Change kind | Version bump |
|---|---|
| Breaking change to a **Stable** public API | MAJOR (1.x → 2.0.0) |
| New backward-compatible feature on Stable surface | MINOR (1.0.x → 1.1.0) |
| Bug fix, docs, internal refactor (no API change) | PATCH (1.0.0 → 1.0.1) |
| Experimental crate (any change) | No semver guarantee — may break at any patch |

Pre-release identifiers (`-rc.N`, `-beta.N`, `-alpha.N`) signal that the API
may still change before the final release. Downstreams that need stability
should pin to a final tag, not an `-rc` tag.

**The 1.0 guarantee covers only the Stable surface listed below.** Internal
crates, experimental crates, and example crates carry no compatibility promise.

---

## Stable supported surface

The following crates form the **Stable** 1.0 API. Breaking changes to any
public item exported by these crates will require a MAJOR version bump.

| Crate | Description |
|---|---|
| `rf-core` | Core error types, `AppResult`, request/response primitives |
| `rf-web` | Axum integration, middleware, CSRF, security headers, session |
| `rf-validation` | Typed validation DSL, `Validate` trait, 48+ built-in rules |
| `rf-validation-derive` | `#[derive(Validate)]` proc-macro |
| `rf-auth` | JWT extractor, `AuthManager`, session-backed auth |
| `rf-config` | TOML/env configuration loader |
| `rf-routing` | Route builder, `Router` wrapper, `routes!` macro |
| `rf-request` | `ValidatedJson<T>`, `FromRequest` impls, implicit-request globals |
| `rf-response` | `JsonResponse`, `Redirect`, response builders |
| `rf-orm` | SeaORM wrapper, `DatabaseManager`, migration runner |
| `rf-mail` | `Mailer` trait, SMTP/SendGrid/Mailgun backends |
| `rf-cache` | `Cache` trait, Memory/Redis/File backends |
| `rf-jobs` | `Job` trait, `Dispatcher`, retry/DLQ/scheduler |
| `rf-events` | `Event` trait, `EventDispatcher` |
| `rf-facades` | Consolidated re-exports of all facades |
| `rf` | Umbrella prelude crate — `use rf::prelude::*` |
| `rustforge` | Alternative umbrella crate — `use rustforge::*` |

**Stability caveat for 1.0.0-rc.1:** Several Stable crates have real engines
that are not yet fully wired to their facade/sugar layer (see VISION_GAP.md).
The compile surface is stable; the runtime behavior of some facades is still
converging toward production quality. Specific known gaps are documented in
`VISION_GAP.md` and the Tier 4 section therein.

---

## Experimental surface

The following crates are **Experimental** and carry no compatibility
guarantee in 1.x. They may change API, be renamed, merged, or removed
without a MAJOR bump.

Experimental crates are tracked in the `members` list of the workspace
`Cargo.toml` but are **excluded from `default-members`** so a plain
`cargo build` does not pull them in.

| Crate | Status |
|---|---|
| `rf-nova` / `rf-nova-macros` | Nova-style admin panel, early prototype |
| `rf-swagger` | OpenAPI / utoipa integration, incomplete |
| `rf-telescope` | Debug dashboard, skeletal |
| `rf-cms` | Content management system, not production-ready |
| `rf-breeze` | Auth scaffolding code generator |
| `rf-vite` | Vite asset pipeline integration |
| `rf-livereload` | Live-reload / HMR, dev-only tool |
| `rf-socialite` | OAuth provider integration, alpha |
| `rf-cashier` | Stripe billing integration, alpha |
| `rf-mcp` | AI MCP integration, alpha |
| `rf-nightwatch` | Observability dashboard, skeletal |
| `rf-ai` | Anthropic AI provider, alpha |
| `rf-vector` | Vector search, alpha |
| `rf-graphql` | GraphQL via async-graphql, alpha |
| `rf-dusk` | Browser testing, skeletal |
| `rf-sail` | Docker workflow, skeletal |
| `rf-spark` | SaaS billing, skeletal |

**Internal crates** (no public API guarantee, subject to removal):
`rf-application`, `rf-domain`, `rf-infra`, `rf-plugins`, `rf-api`,
`rf-command-executor`, `rf-command-events`, `rf-command-pipeline`,
`rf-signal-handler`, `rf-observability`, `rf-advanced-input`,
`rf-stub-system`, `rf-verbosity`, `rf-tinker-enhanced`, `rf-maintenance`,
`rf-env` (internal env utils, separate from user-facing config),
`foundry-cli` (legacy CLI kept for migration), and all `tests/*` crates.

---

## MSRV policy

### Declared MSRV

**Minimum Supported Rust Version: 1.79.0**

This MSRV is declared via `rust-version = "1.79.0"` in the key public crates
(see `[workspace.package]` in the root `Cargo.toml`, and the individual crate
`Cargo.toml` files for `rf-core`, `rf-web`, `rf-validation`, `rf-macros`,
`rf-routing`, and the `rf` / `rustforge` umbrella crates).

When a user's toolchain is older than 1.79.0, `cargo` will refuse to build
and print a clear "toolchain is older than MSRV" error.

### Tested toolchain

The workspace is **pinned to Rust 1.96.0** via `rust-toolchain.toml`. All CI
jobs and local `cargo check --workspace` runs use this pinned toolchain.
1.79.0 is the MINIMUM we claim to support; 1.96.0 is the version we
actually test on.

The CI `msrv` job (`.github/workflows/ci.yml`) verifies the Stable surface
compiles on Rust 1.79.0 using `dtolnay/rust-toolchain@1.79.0`. It does not
run `-Dwarnings` because newer lints may not exist on older compilers.

### MSRV bump policy

Bumping the MSRV is treated as a **MINOR** version bump (not MAJOR) in 1.x,
following common Rust ecosystem practice. An MSRV bump must be documented in
the CHANGELOG and announced in the release notes with at least one release
cycle of notice. The updated `rust-version` field is the authoritative source
of truth; prose documentation here is kept in sync.

---

## Deprecation policy

### How an API gets deprecated

1. **Mark** — Add `#[deprecated(since = "X.Y.Z", note = "Use Foo instead")]`
   to the item in the source. The deprecation message must name the
   replacement or explain why the API is being removed.

2. **Changelog** — Add an entry under `### Deprecated` in the CHANGELOG for
   the same version that introduces the annotation.

3. **Grace period** — A deprecated item must survive at least **one MINOR
   release** (e.g. deprecated in 1.1.0, removable no earlier than 1.2.0).
   For high-impact items that appear in many downstream applications, the
   grace period should be at least **two MINOR releases**.

4. **Remove** — The item is removed in a subsequent release. In 1.x this
   requires a MAJOR bump (2.0.0). In 0.x, a MINOR bump is sufficient.

### Experimental crates

Experimental crates carry no deprecation guarantee. Items may be removed
without the grace period above. Consumers of experimental crates must pin to
an exact git revision and test upgrades themselves.

---

## Downstream consumption

Because RustForge is **not published to crates.io**, downstreams consume it
via git or path dependencies.

### Git dependency (recommended for published apps)

Add to your project's `Cargo.toml`:

```toml
[dependencies]
rf = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
# — or the umbrella crate —
rustforge = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

To track `main` (unstable, not recommended for production):

```toml
[dependencies]
rf = { git = "https://github.com/Chregu12/RustForge", branch = "main" }
```

### Path dependency (for contributors / monorepo embeddings)

If you have cloned the repository or embedded it as a git submodule:

```toml
[dependencies]
rf = { path = "../RustForge/crates/rf" }
rf-core = { path = "../RustForge/crates/rf-core" }
```

### Individual crates

You can depend on individual crates directly rather than the umbrella:

```toml
[dependencies]
rf-validation = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-auth       = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
rf-cache      = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }
```

### Version scheme

All Stable crates share a single version number (currently `1.0.0-rc.1`)
defined in `[workspace.package].version` in the root `Cargo.toml`. Internal
and experimental crates also inherit this version for consistency, even though
they carry no semver guarantee. The rationale: all crates are released
together as a single git tag, so a shared version is simpler for consumers.

---

## Release procedure

Because we do NOT publish to crates.io, the release procedure is:

1. **Update CHANGELOG.md** — Add a `## [X.Y.Z] - YYYY-MM-DD` section at the
   top of the file. Follow the Keep-a-Changelog format.

2. **Update workspace version** — Edit `version = "..."` in
   `[workspace.package]` (root `Cargo.toml`). Also update the version in the
   handful of crates that override the workspace version individually
   (`rustforge`, `rf-macros`, `rf-routing`).

3. **Run the workspace gate** — The tree must be 0-warning clean:
   ```sh
   RUSTFLAGS="-Dwarnings" cargo check --workspace
   cargo check --workspace --all-features
   ```

4. **Run the full test suite** locally:
   ```sh
   cargo test --workspace
   ```

5. **Commit** with message `release: vX.Y.Z` followed by the standard trailer.

6. **Tag** the release commit:
   ```sh
   git tag -a vX.Y.Z -m "RustForge vX.Y.Z"
   ```

7. **Push** the commit and tag:
   ```sh
   git push origin main
   git push origin vX.Y.Z
   ```

8. **Create a GitHub Release** from the tag, pasting the CHANGELOG section as
   the release notes.

---

## Security

Security vulnerabilities must be reported privately before public disclosure.
See [SECURITY.md](../SECURITY.md) for the full threat model, security-relevant
defaults, responsible disclosure process, and the official reporting contact.
