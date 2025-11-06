# RustForge Tier System

> **Feature Organization and Development Priorities**

This document explains RustForge's tier system for organizing features and setting development priorities.

---

## Overview

RustForge organizes features into four tiers based on importance, complexity, and user demand:

- **Core**: Fundamental framework features
- **Tier 1**: Essential features for most applications
- **Tier 2**: Enterprise features for advanced use cases
- **Tier 3**: Nice-to-have features for specialized needs

---

## Core Features

### Purpose
Foundation features that are absolutely required for the framework to function.

### Characteristics
- **Priority**: Highest
- **Stability**: Must be rock-solid
- **Breaking Changes**: Avoid at all costs
- **Test Coverage**: >= 95%

### Features

#### CLI & Code Generation
- Command-line interface
- Code scaffolding system
- Project structure generators
- **LOC**: ~2,500
- **Status**: ✅ Complete

#### Database Management
- Migration system
- Seeding framework
- Connection management
- Query builder integration
- **LOC**: ~1,800
- **Status**: ✅ Complete

#### Tinker REPL
- Interactive console
- Database inspection
- CRUD operations
- SQL execution
- **LOC**: ~1,200
- **Status**: ✅ Complete

#### HTTP Framework
- REST API support
- Routing system
- Middleware pipeline
- Request/Response handling
- **LOC**: ~2,000
- **Status**: ✅ Complete

#### Authentication Base
- User authentication
- Password hashing
- Session management
- **LOC**: ~1,500
- **Status**: ✅ Complete

---

## Tier 1: Essential Features

### Purpose
Features that most applications will need, providing significant value with moderate complexity.

### Characteristics
- **Priority**: High
- **Target Audience**: 80% of applications
- **Implementation Time**: 2-4 weeks per feature
- **Maintenance**: Moderate effort

### Features

#### Mail System
**Purpose**: Send transactional emails

**Why Tier 1**:
- Nearly every application sends emails
- Critical for user communication
- Common use case: welcome emails, password resets

**Implementation**:
- SMTP integration
- Template support
- Queue integration
- Multiple drivers

**LOC**: 1,809
**Status**: ✅ Complete

**Usage**:
```rust
Mail::to("user@example.com")
    .template("welcome")
    .send().await?;
```

#### Notifications
**Purpose**: Multi-channel notification system

**Why Tier 1**:
- Modern apps need multiple communication channels
- User engagement critical
- Flexible delivery mechanisms

**Channels**:
- Email
- SMS
- Slack
- Push notifications
- Database

**LOC**: 2,234
**Status**: ✅ Complete

#### Task Scheduling
**Purpose**: Automated recurring tasks

**Why Tier 1**:
- Every app has background tasks
- Essential for maintenance jobs
- Critical for automation

**Features**:
- Cron-based scheduling
- Timezone support
- Job dependencies
- Failure handling

**LOC**: 1,567
**Status**: ✅ Complete

#### Caching Layer
**Purpose**: Performance optimization through caching

**Why Tier 1**:
- Performance is critical
- Reduces database load
- Improves response times

**Drivers**:
- Redis (distributed)
- File (simple deployments)
- Memory (testing)
- Database (fallback)

**LOC**: 1,892
**Status**: ✅ Complete

#### Multi-Tenancy
**Purpose**: Support multiple tenants in single application

**Why Tier 1**:
- SaaS applications are common
- Data isolation is critical
- Reduces infrastructure costs

**Features**:
- Database isolation
- Domain routing
- Tenant configuration
- Cross-tenant queries

**LOC**: 5,078 (largest Tier 1 feature)
**Status**: ✅ Complete

---

## Tier 2: Enterprise Features

### Purpose
Advanced features for complex applications with enterprise requirements.

### Characteristics
- **Priority**: Medium-High
- **Target Audience**: 40% of applications
- **Implementation Time**: 1-3 weeks per feature
- **Maintenance**: Higher complexity

### Features Summary

| Feature | Purpose | LOC | Complexity |
|---------|---------|-----|------------|
| API Resources | Model transformation | 892 | Low |
| Soft Deletes | Logical deletion | 456 | Low |
| Audit Logging | Change tracking | 1,234 | Medium |
| Full-Text Search | Search capabilities | 987 | Medium |
| File Storage | Upload management | 1,123 | Medium |
| Broadcasting | Real-time events | 1,456 | High |
| OAuth/SSO | Third-party auth | 1,678 | Medium |
| Configuration | Dynamic config | 567 | Low |
| Rate Limiting | Request throttling | 678 | Medium |
| i18n | Localization | 789 | Medium |
| GraphQL | GraphQL API | 1,123 | High |
| Testing Utils | Test helpers | 891 | Low |

### Detailed Features

#### API Resources
**Why Tier 2**: Not all apps have APIs, but common for modern apps

**Use Cases**:
- Mobile app backends
- Third-party integrations
- Microservices

```rust
UserResource::collection(users).to_json()
```

#### Soft Deletes
**Why Tier 2**: Not always needed, but very useful for data retention

**Use Cases**:
- Audit trails
- Data recovery
- Compliance requirements

```rust
post.delete().await?;  // Soft delete
post.restore().await?; // Restore
```

#### Audit Logging
**Why Tier 2**: Enterprise requirement, not needed for simple apps

**Use Cases**:
- Compliance (GDPR, SOX)
- Security investigations
- User activity tracking

#### Full-Text Search
**Why Tier 2**: Advanced feature, basic search often sufficient

**Use Cases**:
- Content platforms
- E-commerce
- Documentation sites

#### Broadcasting
**Why Tier 2**: Real-time features are advanced use case

**Use Cases**:
- Chat applications
- Live notifications
- Collaborative editing

#### OAuth/SSO
**Why Tier 2**: Not all apps need third-party auth

**Use Cases**:
- "Sign in with Google"
- Enterprise SSO
- Social login

---

## Tier 3: Nice-to-Have Features

### Purpose
Specialized features that add polish but aren't essential.

### Characteristics
- **Priority**: Medium-Low
- **Target Audience**: 20% of applications
- **Implementation Time**: 1-2 weeks per feature
- **Maintenance**: Lower priority

### Features

#### Admin Panel
**Why Tier 3**: Can be built manually, framework just accelerates

**Value**: Significant time savings for CRUD interfaces

**LOC**: 2,345
**Status**: ✅ Complete

**Use Cases**:
- Internal tools
- Content management
- Quick admin interfaces

#### PDF/Excel Export
**Why Tier 3**: Specialized reporting need

**Value**: Common for business applications

**LOC**: 1,234
**Status**: ✅ Complete

**Use Cases**:
- Report generation
- Data export
- Invoice creation

#### Form Builder
**Why Tier 3**: Forms can be hand-coded

**Value**: Consistency and rapid development

**LOC**: 890
**Status**: ✅ Complete

**Use Cases**:
- Dynamic forms
- Survey builders
- Admin interfaces

#### HTTP Client
**Why Tier 3**: Many alternatives available

**Value**: Convenience and consistency

**LOC**: 678
**Status**: ✅ Complete

**Use Cases**:
- API integrations
- Webhooks
- External service calls

---

## Tier Comparison

### Development Priority

```
Core > Tier 1 > Tier 2 > Tier 3
```

### Stability Requirements

```
Core (99%+) > Tier 1 (95%+) > Tier 2 (90%+) > Tier 3 (85%+)
```

### Test Coverage

```
Core (>= 95%) > Tier 1 (>= 90%) > Tier 2 (>= 85%) > Tier 3 (>= 75%)
```

### Breaking Changes

- **Core**: Requires major version bump
- **Tier 1**: Requires minor version bump + migration guide
- **Tier 2**: Can be done in minor version with deprecation
- **Tier 3**: More flexibility for changes

---

## Tier Decision Criteria

### Questions to Determine Tier

1. **Is it required for basic functionality?** → Core
2. **Will 80%+ of applications use it?** → Tier 1
3. **Is it an enterprise/advanced feature?** → Tier 2
4. **Is it nice-to-have but not essential?** → Tier 3

### Examples

**Caching Layer**:
- Required by 80%+ apps? **Yes**
- Essential for performance? **Yes**
- **Decision**: Tier 1

**Admin Panel**:
- Required by 80%+ apps? **No**
- Can be built manually? **Yes**
- Saves significant time? **Yes**
- **Decision**: Tier 3

**OAuth**:
- Required by 80%+ apps? **No** (~40%)
- Enterprise need? **Yes**
- Complex integration? **Yes**
- **Decision**: Tier 2

---

## Feature Lifecycle

### 1. Planning
- Identify user need
- Determine tier
- Estimate complexity
- Design API

### 2. Implementation
- Write tests first (TDD)
- Implement feature
- Write documentation
- Review code

### 3. Release
- Add to changelog
- Update documentation
- Announce feature
- Gather feedback

### 4. Maintenance
- Fix bugs
- Add improvements
- Maintain tests
- Update docs

---

## Statistics (v0.2.0)

### Code Distribution

- **Core**: ~8,000 LOC (33%)
- **Tier 1**: ~12,600 LOC (52%)
- **Tier 2**: ~11,400 LOC (47%)
- **Tier 3**: ~5,147 LOC (21%)

### Implementation Status

- **Core**: 5/5 features ✅ (100%)
- **Tier 1**: 5/5 features ✅ (100%)
- **Tier 2**: 10/10 features ✅ (100%)
- **Tier 3**: 4/4 features ✅ (100%)

**Total**: 24/24 features complete

---

## Future Tiers

### Potential Tier 1 Additions
- API Versioning
- Database Replication Support
- Health Checks & Monitoring

### Potential Tier 2 Additions
- Server-Sent Events (SSE)
- CQRS Support
- Event Sourcing

### Potential Tier 3 Additions
- Payment Processing Integration
- SMS Provider Abstraction
- Image Processing

---

## Conclusion

The tier system helps:

1. **Prioritize Development**: Focus on high-impact features first
2. **Set Expectations**: Users know what to expect from each tier
3. **Manage Complexity**: Different tiers have different quality bars
4. **Guide Contributions**: Contributors understand priorities
5. **Plan Roadmap**: Clear path for future development

---

## Related Documentation

- [Architecture Guide](ARCHITECTURE.md)
- [Features Overview](FEATURES.md)
- [Command Reference](COMMANDS.md)
- [Main README](../README.md)

---

*Last Updated: 2025-11-06*
*RustForge v0.2.0*
