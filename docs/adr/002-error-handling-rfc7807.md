# ADR-002: Error Handling (RFC 7807 Problem Details)

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Produktionsfähige APIs benötigen:
- Konsistente Fehlerformate (Client-parsebar)
- Debuggability (trace_id, stack traces in dev)
- HTTP-Status-Code-Semantik
- Keine Informationslecks (production vs development)

## Decision

**RFC 7807 "Problem Details for HTTP APIs"** als Standard-Fehlerformat

### Format:

```json
{
  "type": "https://api.example.com/errors/validation-failed",
  "title": "Validation Failed",
  "status": 422,
  "detail": "The 'email' field is required",
  "instance": "/api/users",
  "trace_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": {
    "email": ["required", "must be valid email"]
  }
}
```

### Core Error Type:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Validation failed: {0}")]
    Validation(ValidationErrors),

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn to_problem_details(&self, trace_id: &str, instance: &str) -> ProblemDetails {
        // ... RFC 7807 mapping
    }
}
```

## Consequences

**Positiv:**
- ✅ Standard-konform (RFC 7807)
- ✅ Client-Libraries können parsen
- ✅ Trace-ID für Log-Korrelation
- ✅ Development-freundlich (detail fields)

**Negativ:**
- ❌ Etwas verbose (vs plain `{"error": "..."}`)
- ❌ Zusätzliche Struktur-Typen nötig

## Implementation

```rust
// In rf-core/src/error.rs
pub struct ProblemDetails {
    pub type_uri: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub instance: String,
    pub trace_id: String,
    pub extensions: HashMap<String, serde_json::Value>,
}

// Axum Integration
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let ctx = /* get from request extensions */;
        let problem = self.to_problem_details(ctx.trace_id(), ctx.path());

        (StatusCode::from_u16(problem.status).unwrap(),
         Json(problem)).into_response()
    }
}
```
