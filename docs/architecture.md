# Foundry Core – Architektur Blueprint

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

## Nächste Iterationen
- `docs/use-cases.md`: detaillierte Command-Flows.
- `docs/templates.md`: Konventionen & Template-Schema.
- Technische RFCs für Plug-in-API & MCP-Integration.

