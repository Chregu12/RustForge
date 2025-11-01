# Foundry Quickstart

## Voraussetzungen
- Rust `1.78+` mit installiertem `cargo`
- SQLite (`sqlite3`) für die Standard-Adapter
- Optional: eine `.env`, um Pfade (z. B. `FOUNDRY_DOMAIN_MODELS`) oder `DATABASE_URL` zu setzen

## Installation & Überblick
```bash
git clone https://github.com/<ihr-org>/Rust_DX-Framework.git
cd Rust_DX-Framework
cargo run -p foundry-cli -- list
```
Der `list`-Aufruf zeigt alle registrierten Commands inklusive `migrate`, `seed` und der Scaffolding-Werkzeuge.

## Funktionsumfang auf einen Blick
- **CLI-Kommandos**: Migrationen/Seeds durchführen (`migrate`, `seed`, `migrate:refresh`) sowie Artefakte scaffolden (`make:model`, `make:controller`, `make:middleware`) mit Optionen für `--dry-run` und `--force`.
- **HTTP/MCP-Server**: Start via `foundry-cli serve`; stellt `/health`, `/commands`, `/invoke` und einen Multipart-`/upload`-Endpoint bereit (50 MB Limit, Whitelist inkl. `text/plain`) und kann optional über `--mcp-stdio` an ein MCP-Gateway angebunden werden.
- **Dateispeicher**: Uploads landen über den `FileService` auf dem konfigurierten Storage (Standard: `storage/app` bzw. `public/storage`) und liefern direkt eine öffentliche URL zurück.
- **Audit-Logging**: Jede CLI-Interaktion wird im JSONL-Format unter `.foundry/audit.log` mit Status, Nachricht und Nutzdaten protokolliert.
- **Konfigurierbarkeit**: Storage-, Datenbank- und Pfad-Einstellungen können per `.env` oder `STORAGE_CONFIG` überschrieben werden, sodass lokale und Container-Deployments unterstützt werden.

## Datenbank-Migrationen & Seeds
### Migrationen prüfen und anwenden
```bash
# Preview ohne Schreibzugriff
cargo run -p foundry-cli -- --dry-run migrate

# Migrationen ausführen
cargo run -p foundry-cli -- migrate
```

### Seeds deterministisch ausführen
```bash
# Preview der geplanten Seeds
cargo run -p foundry-cli -- --dry-run seed

# Seeds anwenden
cargo run -p foundry-cli -- seed
```
Die Seed-Ausführung liest standardmäßig `seeds/*.sql`, führt sie in alphabetischer Reihenfolge aus und protokolliert jede Seed-Datei in der Tabelle `foundry_seeds`.

### Artisan-ähnliche Kombi-Commands
```bash
# Migrationen ausführen und direkt seeden
cargo run -p foundry-cli -- migrate:seed

# Datenbank zurücksetzen und erneut migrieren (optional mit --seed)
cargo run -p foundry-cli -- migrate:refresh --seed
```
`migrate:refresh` rollt nacheinander alle Migrationen zurück und führt sie anschließend erneut aus. Mit `--seed` werden danach automatisch Seeds gestartet.

## Scaffolding
```bash
# Domain-Modell + Migration erzeugen
cargo run -p foundry-cli -- make:model Account

# REST-Controller + Routes für Account erzeugen
cargo run -p foundry-cli -- make:controller Account

# HTTP-Middleware scaffolden (z. B. Token-Check)
cargo run -p foundry-cli -- make:middleware EnsureToken
```
Die Generatoren erzeugen lauffähige Artefakte und verdrahten automatisch `mod.rs`-Dateien (z. B. `domain/mod.rs`, `app/http/mod.rs`, `app/http/middleware/mod.rs`), sodass neue Module direkt importierbar sind. Zusätzlich wird `app/http/kernel.rs` als zentraler Einstieg mit einem vorkonfigurierten (durchleitenden) `global_middleware`-Hook angelegt, über den Sie globale Guards oder Logging ergänzen können. Mit `--dry-run` erhalten Sie eine Vorschau ohne Dateien zu schreiben, `--force` überschreibt vorhandene Artefakte.

## HTTP/MCP Server starten
```bash
cargo run -p foundry-cli -- serve --addr 127.0.0.1:8080
```
Der Server bietet `/health`, `/commands` und `/invoke`. Optional kann `--mcp-stdio` aktiviert werden. Über `HttpServer::merge_router` und `HttpServer::with_middleware` lassen sich eigene Router und Guards einklinken; JSON-Payloads werden über `AppJson<T>` automatisch validiert und mit `JsonResponse<T>` serialisiert zurückgegeben.

## Audit-Log
Nach jedem Kommando wird ein JSONL-Eintrag unter `.foundry/audit.log` (Override via `FOUNDRY_AUDIT_LOG`) angelegt. Enthalten sind Zeitstempel, Kommando, Exit-Status sowie eventuelle Nutzdaten.

## Relevante Environment-Variablen
| Variable | Zweck | Standard |
|----------|-------|-----------|
| `DATABASE_URL` | Ziel-Datenbank für Migrationen/Seeds | `sqlite::memory:` |
| `FOUNDRY_DOMAIN_MODELS` | Ablage für generierte Domain-Modelle | `domain/models` |
| `FOUNDRY_MIGRATIONS` | Pfad zu SQL-Migrationen | `migrations` |
| `FOUNDRY_HTTP_CONTROLLERS` | Controller-Verzeichnis | `app/http/controllers` |
| `FOUNDRY_HTTP_ROUTES` | Route-Module | `app/http/routes` |
| `FOUNDRY_HTTP_MIDDLEWARE` | Middleware-Module | `app/http/middleware` |
| `FOUNDRY_AUDIT_LOG` | Zielpfad für JSONL-Audit | `.foundry/audit.log` |

Damit ist ein neues Projekt lauffähig: Migrationen/Seeds vorbereiten, gewünschte Module scaffolden, Server starten – jede Aktion wird nachvollziehbar im Audit-Log erfasst.
