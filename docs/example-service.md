# Beispiel-Service: Account-Verwaltung

Dieses Beispiel demonstriert, wie Sie mit Foundry in wenigen Schritten einen vollständigen Account-Service aufsetzen – inklusive Domain-Modell, Migration, Seed-Daten und HTTP-Endpunkt.

## 1. Domain-Modell & Migration scaffolden
```bash
cargo run -p foundry-cli -- make:model Account
```
Ergebnis:
- `domain/models/account.rs` mit einer `Account`-Struktur
- `migrations/<timestamp>_create_account_table/{up,down}.sql`
- Automatisch erzeugte `mod.rs`-Dateien (`domain/mod.rs`, `domain/models/mod.rs`)

## 2. Migration anwenden
```bash
cargo run -p foundry-cli -- migrate
```
Damit wird die Tabelle `account` angelegt. Der Dry-Run (`--dry-run`) zeigt zuvor den Plan.

## 3. Seed-Daten hinterlegen
Legen Sie zusätzliche SQL-Dateien im Verzeichnis `seeds/` an oder passen Sie die mitgelieferte `20251027180000_bootstrap.sql` an.
```bash
cargo run -p foundry-cli -- seed
```
Seeds werden alphabetisch sortiert, in einer Transaktion ausgeführt und in der Tabelle `foundry_seeds` protokolliert.

> Shortcut: `cargo run -p foundry-cli -- migrate:seed` kombiniert Migration und Seed in einem Schritt. Für einen kompletten Reset nutzen Sie `cargo run -p foundry-cli -- migrate:refresh --seed`.

## 4. HTTP-Controller erzeugen
```bash
cargo run -p foundry-cli -- make:controller Account
```
Dies generiert:
- `app/http/controllers/account_controller.rs` mit CRUD-Stubs
- `app/http/routes/account.rs` für die Routenregistrierung
- Modulverkabelung (`app/mod.rs`, `app/http/mod.rs`, `app/http/controllers/mod.rs`, `app/http/routes/mod.rs`)
- `app/http/kernel.rs` als HTTP-Einstieg, der globale Middleware über `global_middleware` erlaubt

Optional können Sie ergänzend Guards scaffolden:
```bash
cargo run -p foundry-cli -- make:middleware EnsureToken
```
Die Middleware landet unter `app/http/middleware/ensure_token.rs` und lässt sich per `HttpServer::with_middleware` aktivieren.

## 5. Projekt mit eigener Top-Level-Modulstruktur verbinden
In Ihrer eigenen `src/lib.rs` oder `src/main.rs` genügt anschließend:
```rust
pub mod app;      // generiert vom Scaffold
pub mod domain;   // enthält Account-Domain
```
Die generierten Module sind sofort importierbar, etwa:
```rust
use crate::app::http::controllers::account_controller;
use crate::domain::models::account::Account;
```

## 6. Server starten
```bash
cargo run -p foundry-cli -- serve --addr 127.0.0.1:8080
```
Über `/commands` sehen Sie alle verfügbaren Commands; neue Aufrufe werden im Audit-Log (`.foundry/audit.log`) dokumentiert.

## Weiterführende Ideen
- Routen mit Axum-Routern kombinieren (`account::routes()` in Ihre App integrieren)
- Seeds erweitern, um Demo-Daten vorzubereiten
- `make:migration` für weitere Tabellen nutzen
- Middleware-Guards (`make:middleware`) in `app/http/middleware` ablegen und im Server via `with_middleware` aktivieren
- Eigene Generatoren über `foundry-plugins` implementieren

Mit diesen Bausteinen steht ein funktionsfähiges Grundgerüst bereit, das deterministische Migrationen/Seeds, generierte Module und ein vollständiges Audit-Log umfasst.
