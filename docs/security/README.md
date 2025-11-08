# Security Documentation

Welcome to the Foundry Framework security documentation. This directory contains comprehensive guides for implementing and understanding security features.

## Quick Links

- **[Security Best Practices](./SECURITY_BEST_PRACTICES.md)** - Start here for overall security guidelines
- **[Security Audit Report](../../SECURITY_AUDIT.md)** - Comprehensive security audit findings
- **[Implementation Summary](../../SECURITY_IMPLEMENTATION_SUMMARY.md)** - Overview of all security features

## Guides

### Authentication & Authorization

- **[Authorization (Gates & Policies)](./AUTHORIZATION.md)**
  - Gates for general abilities
  - Policies for resource-specific permissions
  - Before/after hooks
  - Common patterns

- **[OAuth Setup](./OAUTH_SETUP.md)**
  - Provider configuration (Google, GitHub, Facebook)
  - Integration guide
  - Security best practices
  - Token management

### Protection Mechanisms

- **[CSRF Protection](./CSRF_PROTECTION.md)**
  - Token generation and validation
  - Template integration
  - AJAX/SPA usage
  - Configuration options

- **[Rate Limiting](./RATE_LIMITING.md)**
  - Multiple strategies (IP, User, Route, Custom)
  - Time windows configuration
  - Exemptions and whitelists
  - Client-side handling

### General Security

- **[Security Best Practices](./SECURITY_BEST_PRACTICES.md)**
  - Password requirements
  - Session management
  - Input validation
  - API security
  - File uploads
  - Secrets management
  - Production checklist

## Getting Started

If you're new to Foundry security features, we recommend this learning path:

1. **Read:** [Security Best Practices](./SECURITY_BEST_PRACTICES.md)
2. **Implement:** [CSRF Protection](./CSRF_PROTECTION.md)
3. **Configure:** [Rate Limiting](./RATE_LIMITING.md)
4. **Authorize:** [Gates & Policies](./AUTHORIZATION.md)
5. **Integrate:** [OAuth Setup](./OAUTH_SETUP.md) (if using social auth)
6. **Review:** [Security Audit Report](../../SECURITY_AUDIT.md)

## Security Features Overview

### CSRF Protection

Protect against Cross-Site Request Forgery attacks:

```rust
use foundry_application::middleware::csrf::CsrfMiddleware;

let csrf = CsrfMiddleware::new()
    .exempt("/api/*")
    .exempt("/webhooks/*");
```

**[Full Guide →](./CSRF_PROTECTION.md)**

### Rate Limiting

Prevent abuse and DDoS attacks:

```rust
use foundry_application::middleware::rate_limit::{RateLimitMiddleware, RateLimitConfig};

let limiter = RateLimitMiddleware::in_memory(
    RateLimitConfig::per_ip(60)  // 60 requests per minute
);
```

**[Full Guide →](./RATE_LIMITING.md)**

### Authorization

Control access with Gates and Policies:

```rust
// Gates - General abilities
Gate::define("manage-users", |user| {
    user.is_admin()
}).await;

// Policies - Resource-specific
impl ResourcePolicy<User, Post> for PostPolicy {
    fn update(&self, user: &User, post: &Post) -> bool {
        user.id == post.author_id
    }
}
```

**[Full Guide →](./AUTHORIZATION.md)**

### OAuth Integration

Social authentication with multiple providers:

```rust
let mut oauth = OAuthClient::new();
oauth.register_provider(Box::new(GoogleProvider::new(...)));

let (url, state) = oauth.get_authorize_url("google").await?;
```

**[Full Guide →](./OAUTH_SETUP.md)**

## Security Checklist

Before deploying to production, ensure:

- [ ] All default secrets replaced with strong, random secrets
- [ ] HTTPS enforced
- [ ] CSRF protection enabled
- [ ] Rate limiting configured
- [ ] Input validation implemented
- [ ] SQL injection prevention verified (using ORM)
- [ ] XSS prevention in templates
- [ ] Security headers configured
- [ ] Session security configured
- [ ] Password policy enforced
- [ ] Error messages don't leak sensitive information
- [ ] Logging configured (no sensitive data logged)
- [ ] Dependencies audited (`cargo audit`)
- [ ] Authorization checks in place
- [ ] OAuth providers fully implemented

**[Full Checklist →](./SECURITY_BEST_PRACTICES.md#security-checklist)**

## Security Audit

A comprehensive security audit has been conducted. Key findings:

**Grade:** B+ (GOOD, Production-Ready with Recommendations)

**Strengths:**
- Comprehensive authentication system
- Strong password hashing (Argon2, BCrypt)
- CSRF protection implemented
- Rate limiting with multiple strategies
- Authorization system (Gates & Policies)
- OAuth with state validation

**Critical Items:**
- Complete OAuth provider implementations
- Migrate to Redis for production sessions/rate limiting
- Add security headers middleware

**[Full Report →](../../SECURITY_AUDIT.md)**

## Contributing

When adding security features:

1. Follow secure coding practices
2. Include comprehensive tests
3. Document all security implications
4. Update this index
5. Get security review before merging

## Support

For security-related questions:

- Check the relevant guide in this directory
- Review the [Security Audit Report](../../SECURITY_AUDIT.md)
- Consult [Security Best Practices](./SECURITY_BEST_PRACTICES.md)

For security vulnerabilities:
- DO NOT open a public issue
- Email: security@foundry.rs (TODO: Setup security email)
- Use GitHub Security Advisories

## Additional Resources

- **[OWASP Top 10](https://owasp.org/Top10/)** - Web application security risks
- **[Rust Security Advisory Database](https://rustsec.org/)** - Known vulnerabilities in Rust crates
- **[Cargo Audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)** - Audit Cargo.lock for security issues

---

**Last Updated:** 2025-11-08
**Security Specialist:** Lead Developer 4
