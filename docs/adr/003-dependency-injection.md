# ADR-003: Dependency Injection (Service Registry + Scopes)

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Wir benötigen DI für:
- Testability (Mock-Services injizieren)
- Scoped-Lifetime (app/request/worker)
- Type-safe Service Resolution
- Circular Dependency Detection

## Decision

**Eigene Service Registry** mit Scope-Support (Singleton, Request, Transient)

### API-Design:

```rust
// Service Registration (startup)
let registry = ServiceRegistry::new();

registry.singleton(|| Arc::new(DbPool::connect(&db_url)));
registry.scoped(|ctx: &RequestContext| UserService::new(ctx.db()));
registry.transient(|| AuditLogger::new());

// Service Resolution
let db: Arc<DbPool> = registry.resolve()?;
let user_service: UserService = ctx.resolve()?;
```

### Scope-Semantik:

- **Singleton:** App-Lebensdauer (DB-Pool, Config)
- **Scoped (Request):** Pro HTTP-Request (Services mit Request-Kontext)
- **Transient:** Jedes `resolve()` neue Instanz

### Alternativen (abgelehnt):

**shaku:**
- ❌ Generics-Heavy, schwierige Error-Messages
- ❌ Kein Request-Scope out-of-the-box

**ctor/linkme:**
- ❌ Globale Registrierung, schwer testbar
- ❌ Kein Scope-Konzept

## Consequences

**Positiv:**
- ✅ Type-safe (`resolve::<T>()`)
- ✅ Klare Scopes
- ✅ Testbar (Mock-Registry für Tests)

**Negativ:**
- ❌ Eigene Implementierung = Wartungsaufwand
- ❌ Kein Circular-Dependency-Check zur Compile-Zeit

## Implementation

```rust
// rf-container/src/lib.rs
pub struct ServiceRegistry {
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    scoped: HashMap<TypeId, Box<dyn Fn(&RequestContext) -> Box<dyn Any>>>,
    transient: HashMap<TypeId, Box<dyn Fn() -> Box<dyn Any>>>,
}

impl ServiceRegistry {
    pub fn singleton<T>(&mut self, factory: impl Fn() -> T)
    where T: 'static + Send + Sync
    {
        let instance = Arc::new(factory());
        self.singletons.insert(TypeId::of::<T>(), instance);
    }

    pub fn resolve<T>(&self) -> Result<Arc<T>, ResolveError>
    where T: 'static + Send + Sync
    {
        self.singletons.get(&TypeId::of::<T>())
            .ok_or(ResolveError::NotRegistered)?
            .clone()
            .downcast::<T>()
            .map_err(|_| ResolveError::TypeMismatch)
    }
}
```

### Macro für ergonomisches Registrieren:

```rust
register! {
    app.registry,
    singleton: DbPool,
    scoped: UserService,
    transient: AuditLogger,
}
```
