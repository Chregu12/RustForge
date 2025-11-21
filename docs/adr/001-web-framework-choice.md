# ADR-001: Web Framework Choice (Axum + Tower)

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Wir benötigen ein async-first Web-Framework für produktionsreife Rust-Anwendungen mit:
- Typ-sicheren Handler-Signaturen
- Middleware-Komposition
- Extractor-Pattern für Request-Parsing
- Performance (async/await, Tokio)

## Decision

**Axum** als Web-Framework + **Tower** für Middleware-Stack

### Begründung:

**Axum:**
- ✅ Von Tokio-Team entwickelt, exzellente async Performance
- ✅ Typ-sichere Extractors (kein Reflection)
- ✅ Compiler-gestützte Handler-Validierung
- ✅ Minimale Boilerplate
- ✅ Aktive Community, gute Docs

**Tower:**
- ✅ Middleware als `Service<Request>`-Trait
- ✅ Komponierbar via `ServiceBuilder`
- ✅ Battle-tested (Linkerd, AWS SDK)
- ✅ Request/Response-Transformation klar getrennt

### Alternativen (abgelehnt):

**Actix-web:**
- ❌ Actor-Model erhöht Komplexität
- ❌ Weniger typ-sicher (Any-basierte Extractors)

**Rocket:**
- ❌ Async-Support noch instabil
- ❌ Codegen-Overhead

**Warp:**
- ❌ Filter-Komposition weniger intuitiv
- ❌ Weniger aktive Entwicklung

## Consequences

**Positiv:**
- Typ-Sicherheit zur Compile-Zeit
- Performance-Overhead minimal
- Einfache Testing-Story (Tower-Test Utilities)

**Negativ:**
- Steilere Lernkurve für Tower-Middleware-Konzepte
- Manuelle Error-Mapper nötig (kein global Error Handler)

## Implementation

```rust
// Core Router Setup
use axum::{Router, routing::get};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

Router::new()
    .route("/health", get(health_check))
    .layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(request_id::Layer)
            .layer(cors::Layer)
    )
```
