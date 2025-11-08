# Foundry Core – Architektur Blueprint

> ⚠️ **Status:** v0.2.0 - Architecture document reflects intended design. Actual implementation has gaps (see Known Limitations below).

## Vision
Foundry Core stellt ein kompaktes Rust-Framework bereit, das Laravel-Artisan-DX in ein modular aufgebautes, service-orientiertes Rust-Ökosystem überträgt. Das System liefert einen konsistenten CLI-Einstiegspunkt (`foundry`), stellt API-Schnittstellen für Automatisierung (HTTP/gRPC, MCP) bereit und bleibt durch Domain-Driven-Design klar strukturiert.

## Leitprinzipien
- **DDD-Schichten** – Trennung in `domain`, `application`, `infrastructure`, `interface`.
- **Bounded Contexts** – CLI-Orchestrierung, Scaffolding, Datenbank, Template-System.
- **Microservice-Ready** – Lose Kopplung via Ports & Adapter, klare Contracts, transportagnostisch.
- **Convention over Configuration** – Standardisierte Projektstruktur, generierte Artefakte folgen Konventionen.
- **DX & Automation** – Menschliche und maschinenlesbare Outputs (JSON/Human), Audit-Trails, Dry Runs.

## Workspace-Struktur (vorgeschlagen)

| Crate                | Schicht/Context         | Verantwortung |
|----------------------|-------------------------|---------------|
| `foundry-cli`        | Interface               | CLI-Registry, Output-Formatierung, Prompting |
| `foundry-application`| Application             | Use-Cases, Command-Handler, Orchestrierung |
| `foundry-domain`     | Domain (Kern)           | Aggregates, Policies, Value Objects, Events |
| `foundry-infra`      | Infrastructure          | Persistence (SeaORM), Filesystem, Templating, Audit-Logs |
| `foundry-plugins`    | Extension Boundary      | Trait-basierte Plug-in API für Generatoren & Commands |
| `foundry-api`        | Interface (optional)    | HTTP/gRPC Layer für Remote-Aufrufe, MCP Server Binding |

> Alle Crates liegen in einem `Cargo`-Workspace (`Cargo.toml` im Root). Kontextinterne Module werden nach dem DDD-Schnitt gegliedert (`domain/models`, `application/services`, etc.).

## Aufrufkanäle
- **CLI (`foundry`)** – Default-Einstiegspunkt für lokale Developer-Workflows.
- **Service API** – HTTP/gRPC Endpoints spiegeln Kern-Commands wider, erlauben Microservice-Einbindung.
- **MCP Server** – Bindet Foundry-Kommandos über ein maschinenlesbares Command-Interface für LLM-Agenten ein.

Die Invocations nutzen identische Application-Handler; Adapter/Ports übernehmen Transport-Übersetzungen.

## Bounded Contexts & Use-Cases
- **Core Orchestration** – Registrierung, Discovery & Dispatch von Commands, Feature-Fahnen, Telemetrie.
- **Environment Management** – `.env`-Handling, Secrets-Resolver, Config-Merging.
- **Database Lifecycle** – Migration Runner, Rollbacks, Seeder, Factory Orchestrierung (SeaORM).
- **Scaffolding & Templates** – Codegeneratoren, Templating-Engine, Project Blueprints, File Writers.
- **Interaction Layer** – Prompting, Progress Feedback, Output Renderer (JSON/Human, Audit-Trails).

## Erweiterungen & Modularität
- Commands & Generatoren implementieren das `Command`/`Generator`-Trait aus `foundry-plugins`.
- Plug-ins können als separate Crates eingebunden werden, registrieren sich über Discovery (Cargo Features, Config, oder Directory-Scanning).
- Infrastruktur-Adapter (z. B. für unterschiedliche ORMs) werden über Ports abstrahiert (`DatabasePort`, `TemplatePort`).

## Daten- & Kontrollfluss
1. Interface (CLI/API/MCP) parst Eingaben → Application Command.
2. Application-layers orchestrieren Domain-Services und Ports.
3. Domain trifft Entscheidungen (Policies, Validierung), emittiert Events.
4. Infrastruktur-Adapter persistieren oder erzeugen Artefakte.
5. Interface rendert Response (inkl. Audit-Eintrag).

## Known Limitations (v0.2.0)

### 1. Production Backends Missing
**Current State:** All backends (queue, cache, events) use in-memory implementations only.
**Impact:**
- Cannot scale horizontally across multiple instances
- Data lost on restart
- Not suitable for production deployments
**Timeline:** v0.3.0 (December 2025) - Redis backends for queue and cache

### 2. Validation System Incomplete
**Current State:** Basic validation structure exists, but only stub implementations.
**Impact:**
- Manual validation required for most use cases
- No built-in validation rules (email, required, min, max, etc.)
- FormRequest pattern not implemented
**Timeline:** v0.3.0 (December 2025) - 20+ validation rules, FormRequest integration

### 3. Security Features Partial
**Current State:** Basic authentication works (JWT, sessions), but critical security features missing.
**Missing:**
- CSRF protection
- Rate limiting
- Authorization (Gates & Policies)
- OAuth completion (providers partially implemented)
- Security headers middleware
**Impact:** Not secure for production use
**Timeline:** v0.3.0 (December 2025) - CSRF, rate limiting, Gates & Policies

### 4. Test Coverage Gaps
**Current State:** ~50% test coverage, some tests have compilation errors.
**Issues:**
- `foundry-http-client` tests fail to compile (accessing private fields)
- Integration test gaps
- No end-to-end tests
- Performance benchmarks incomplete
**Impact:** Unknown bugs may exist, regression risk high
**Timeline:** v0.3.0 (December 2025) - >70% coverage, all tests passing

### 5. ORM Limited
**Current State:** Sea-ORM is integrated but lacks high-level abstractions.
**Missing:**
- Eloquent-style model API
- Relationship definitions (hasMany, belongsTo, etc.)
- Eager loading (N+1 query prevention)
- Query scopes
- Model events (creating, created, updating, etc.)
**Impact:** More boilerplate code, less Laravel-like DX
**Timeline:** v0.4.0 (2026) - Eloquent-style API, relationships

### 6. Documentation-Code Mismatch
**Current State:** Documentation claims features that are incomplete or stub-only.
**Examples:**
- OAuth listed as "✅" but only partially implemented
- Admin panel listed as complete but needs work
- Production-ready claim inaccurate
**Impact:** User confusion, incorrect expectations
**Timeline:** v0.3.0 (December 2025) - Honest documentation (this update)

### 7. No Production Deployments
**Current State:** Framework has not been deployed to production anywhere.
**Impact:**
- No real-world battle-testing
- Unknown performance characteristics at scale
- Deployment procedures untested
- No production troubleshooting guides
**Timeline:** Post v1.0.0 (2026) - Community production deployments expected

## Nächste Iterationen
- `docs/use-cases.md`: detaillierte Command-Flows.
- `docs/templates.md`: Konventionen & Template-Schema.
- Technische RFCs für Plug-in-API & MCP-Integration.
- `/TEAM_COORDINATION.md`: Comprehensive team coordination and development plan (completed)
- `/docs/PRODUCTION_BACKENDS.md`: Redis queue/cache implementation guide (v0.3.0)
- `/docs/VALIDATION.md`: Validation system documentation (v0.3.0)
- `/docs/SECURITY.md`: Security best practices and implementation (v0.3.0)

