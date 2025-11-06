# RustForge Test Application

Dies ist ein Testprojekt für das RustForge Framework.

## Features in diesem Test

1. **Framework Bootstrap** - Initialisierung aller Framework-Komponenten
2. **Command Execution** - Ausführung von Framework-Commands
3. **HTTP Server** - REST API für Command-Invokation
4. **Dependency Injection** - Service Container und Providers

## Build und Run

```bash
# Build
cargo build

# Run
cargo run

# Release Build
cargo build --release
```

## API Endpoints

Nach dem Start des Servers sind folgende Endpoints verfügbar:

### Health Check
```bash
curl http://localhost:8080/health
```

### List Commands
```bash
curl http://localhost:8080/api/commands
```

### Invoke Command
```bash
curl -X POST http://localhost:8080/api/invoke \
  -H 'Content-Type: application/json' \
  -d '{
    "command": "list",
    "args": [],
    "format": "json"
  }'
```

## Was wird getestet?

- ✅ Framework Bootstrap (FoundryApp)
- ✅ Command Registry
- ✅ Command Execution (list, test, etc.)
- ✅ HTTP Server
- ✅ API Endpoints
- ✅ JSON Serialization
- ✅ Error Handling
- ✅ Tracing/Logging

## Projekt-Struktur

```
test-project/
├── Cargo.toml          # Dependencies und Build-Konfiguration
├── .env                # Umgebungsvariablen
├── README.md           # Diese Datei
└── src/
    └── main.rs         # Hauptanwendung
```
