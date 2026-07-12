# RustForge Documentation

This directory contains the documentation for the RustForge framework. Every kept file
reflects the **verified state** documented in `VISION_GAP.md` and the maturity matrix
in the top-level `README.md`. No document here claims 100% completion or production
certification beyond what is graded Stable/Usable in those sources.

> **Feature-tier taxonomy:** [`docs/TIERS.md`](./TIERS.md) is the **single canonical
> source** for every workspace crate's maturity tier (`stable` / `beta` /
> `experimental` / `stub`). The README maturity matrix is a user-facing summary that
> maps onto the same tier definitions. Consult `TIERS.md` before making any maturity
> claim in documentation.

> **Experimental crates** (`rf-nova`, `rf-nova-macros`, `rf-swagger`, `rf-telescope`,
> `rf-cms`, `rf-breeze`, `rf-vite`, `rf-livereload`) are **not covered by SemVer
> guarantees** and are excluded from the workspace `default-members`. They compile
> under `cargo check --workspace` (no bitrot) but are skipped by plain `cargo check`.
> See the "Experimental crates" table in [`docs/TIERS.md`](./TIERS.md).

---

## Start here

| File | Purpose |
|------|---------|
| [TIERS.md](./TIERS.md) | **Canonical tier taxonomy** — every workspace crate (113 total) with its `stable`/`beta`/`experimental`/`stub` tier and one-line justification. Read before making any maturity claim. |
| [GETTING_STARTED.md](./GETTING_STARTED.md) | 5-minute quickstart, full REST resource example, validated-DTO pattern, maturity matrix with per-surface notes. **Read this first.** |
| [COOKBOOK.md](./COOKBOOK.md) | Task-oriented recipes for every verified surface (routing, ORM, auth, jobs, broadcast, mail, i18n, health, CLI, deploy…). Every snippet is grep-verified against the source. |
| [EXAMPLES.md](./EXAMPLES.md) | Gallery of runnable, CI-tested examples with descriptions. |
| [installation.md](./installation.md) | Prerequisites, Forge CLI install, and project creation. |

---

## Reference

| File | Purpose |
|------|---------|
| [ERROR_CODES.md](./ERROR_CODES.md) | RF001–RF999 error code reference for all framework domains. |
| [DEPLOYMENT_GUIDE.md](./DEPLOYMENT_GUIDE.md) | Production deployment: environment config, Docker, Kubernetes, monitoring. |
| [PRODUCTION_BACKENDS.md](./PRODUCTION_BACKENDS.md) | Migrating from in-memory backends to Redis for cache (`rf-cache`) and queue (`rf-queue`). |

---

## Laravel-style helpers

| File | Purpose |
|------|---------|
| [LARAVEL_SYNTAX.md](./LARAVEL_SYNTAX.md) | Full status table: what is functional (Hash, csrf_token, rules!, validate!), what is metadata-only (Route string facade), and the recommended real-app path. |
| [LARAVEL_SYNTAX_QUICK_START.md](./LARAVEL_SYNTAX_QUICK_START.md) | 5-minute tour of Hash, csrf_token, rules!, and validate! helpers. |

---

## CLI / foundry-cli

| File | Purpose |
|------|---------|
| [quickstart.md](./quickstart.md) | foundry-cli quickstart: migrate, seed, scaffold commands. |

---

## Architecture and design decisions

| Location | Purpose |
|----------|---------|
| [architecture.md](./architecture.md) | Current architecture: four pillars, crate map by layer, request flow, AsyncBridge pattern, design ceilings, ADR summary. Supersedes the old v0.2.0 "Foundry Core" blueprint. |
| [adr/](./adr/) | Architecture Decision Records: web framework (axum 0.8), error handling (RFC 7807), DI, observability, config, ORM (SeaORM), queue (Redis). All active; none contradicts current implementation. |

---

## Guides

| Location | Purpose |
|----------|---------|
| [guides/database-persistence.md](./guides/database-persistence.md) | Database persistence guide. |
| [guides/migration.md](./guides/migration.md) | Migration guide. |
| [guides/websocket.md](./guides/websocket.md) | WebSocket / Broadcast guide. |
| [guides/graphql.md](./guides/graphql.md) | GraphQL integration guide. |
| [guides/quick-start-performance.md](./guides/quick-start-performance.md) | Performance quick-start. |

---

## Security

| Location | Purpose |
|----------|---------|
| [security/README.md](./security/README.md) | Security overview index. |
| [security/SECURITY_BEST_PRACTICES.md](./security/SECURITY_BEST_PRACTICES.md) | Security best practices checklist. |
| [security/AUTHORIZATION.md](./security/AUTHORIZATION.md) | Authorization (gates, policies, RBAC). |
| [security/CSRF_PROTECTION.md](./security/CSRF_PROTECTION.md) | CSRF protection. |
| [security/RATE_LIMITING.md](./security/RATE_LIMITING.md) | Rate limiting. |
| [security/OAUTH_SETUP.md](./security/OAUTH_SETUP.md) | OAuth provider setup. |

---

## Deployment

| Location | Purpose |
|----------|---------|
| [deployment/guide.md](./deployment/guide.md) | Detailed deployment guide. |
| [deployment/docker.md](./deployment/docker.md) | Docker-specific deployment. |

---

## Development

| Location | Purpose |
|----------|---------|
| [development/testing.md](./development/testing.md) | Testing guide. |
| [development/benchmarks.md](./development/benchmarks.md) | Benchmarking guide. |
| [development/metrics.md](./development/metrics.md) | Metrics and observability development guide. |

---

## Wiki

| Location | Purpose |
|----------|---------|
| [wiki/README.md](./wiki/README.md) | Wiki index. |
| [wiki/Laravel-Syntax.md](./wiki/Laravel-Syntax.md) | Honest summary of Laravel-style helpers and what is vs. is not functional. |
| [wiki/Home.md](./wiki/Home.md) | Wiki home. |
| [wiki/Features.md](./wiki/Features.md) | Feature overview (may be stale — cross-check with README maturity matrix). |

---

## Tutorials and Snippets

| Location | Purpose |
|----------|---------|
| [tutorials/](./tutorials/) | Step-by-step tutorials. |
| [snippets/](./snippets/) | Code snippets: authentication, database. |

---

## Grounding documents (repo root, not this directory)

| File | Purpose |
|------|---------|
| `README.md` | North-star: maturity matrix, quickstart, known limitations. |
| `VISION_GAP.md` | Full audit of implementation vs. vision; 16 areas graded; roadmap. |
| `CONTRIBUTING.md` | How to contribute; stub-hunt process; CI requirements. |
| `CHANGELOG.md` | Version history. |

---

## What is not here (intentionally removed)

The following files were removed because they contradicted the honest verified state:

- `100_PERCENT_PARITY_REPORT.md` — falsely claimed 100% Laravel 12 feature parity
- `PRODUCTION_READINESS_CHECKLIST.md` — falsely claimed 100/100 production certification
- `WIKI_UPDATE.md` — falsely claimed a completed fully-sync "Phase 21" API
- `FEATURES.md` — used obsolete `foundry_*` crate names; claimed all features complete
- `COMMANDS.md` — used obsolete `foundry` CLI command names
- `BEST_PRACTICES.md` — contained incorrect API signatures and unreachable crate names
- `METRICS.md` — used obsolete `foundry_infra` crate names; written in German
- `LARAVEL_SYNTAX_FIXES_REPORT.md` — stale development notes about a superseded design direction
- `architecture/roadmap.md` — wholesale "Foundry" era crate names throughout; superseded by `VISION_GAP.md` for current priorities
