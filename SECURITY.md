# Security Policy

## Supported Versions

We actively support the following versions of RustForge with security updates:

| Version | Supported          | Status |
| ------- | ------------------ | ------ |
| 1.0.x   | :white_check_mark: | Current stable release |
| 0.2.x   | :x:                | Deprecated (not production-ready) |
| 0.1.x   | :x:                | Deprecated (alpha) |
| < 0.1   | :x:                | Not supported |

**Note:** Only the latest stable version (1.0.x) is production-ready and receives security updates.

---

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### Reporting Process

1. **DO NOT** open a public GitHub issue for security vulnerabilities
2. Email security reports to: **security@rustforge.dev** (or create a private security advisory on GitHub)
3. Include the following information:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
   - Your contact information

### What to Expect

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: 1-7 days
  - High: 7-14 days
  - Medium: 14-30 days
  - Low: 30-90 days

### Disclosure Policy

- We follow **coordinated disclosure**
- Security fixes will be released as soon as possible
- Public disclosure after fix is available
- Credit given to reporter (unless they prefer anonymity)

---

## Security Features

### Authentication & Authorization

**Implemented (v1.0.0):**
- JWT-based authentication (HMAC-SHA256)
- Password hashing (Argon2 by default, Bcrypt supported)
- Email verification with time-limited tokens
- Password reset with one-time tokens
- Remember me with HTTP-only cookies
- Token rotation for enhanced security

**Planned (v1.1.0+):**
- RBAC (Role-Based Access Control)
- Permission system
- OAuth 2.0 server
- Multi-factor authentication (2FA/TOTP)

### Data Security

**Current:**
- SQL injection protection (parameterized queries via Sea-ORM)
- Path traversal prevention (jailed filesystem access)
- XSS protection (HTTP-only cookies, secure flags)
- CSRF protection (SameSite cookies)
- TLS/SSL support for all network connections

**Planned:**
- Content Security Policy (CSP) headers
- HSTS (HTTP Strict Transport Security)
- Subresource Integrity (SRI)
- Audit logging with encryption at rest

### Network Security

**Current:**
- HTTPS enforcement in production
- CORS configuration
- Rate limiting (Redis-backed)
- Connection pooling with limits
- Timeout configurations

**Planned:**
- DDoS protection
- IP whitelisting/blacklisting
- Advanced rate limiting (per-user, per-route)
- Request signing

### Storage Security

**Current:**
- Presigned URLs for temporary S3 access (15min default)
- Credential protection (no hardcoded secrets)
- Path validation and sanitization
- Secure file upload handling

**Planned:**
- Server-side encryption
- Client-side encryption
- File integrity checking
- Virus scanning integration

---

## Security Best Practices

### For Application Developers

1. **Environment Variables**
   ```env
   # NEVER commit .env to version control
   # Use strong, unique secrets (min 32 characters)
   JWT_SECRET=<strong-random-secret-min-32-chars>
   PASSWORD_RESET_SECRET=<different-strong-secret>
   EMAIL_VERIFICATION_SECRET=<another-different-secret>
   REMEMBER_ME_SECRET=<yet-another-secret>
   ```

2. **Password Hashing**
   ```rust
   // Use Argon2 (default, recommended)
   PASSWORD_HASH_ALGORITHM=argon2
   
   // Or Bcrypt for legacy compatibility
   PASSWORD_HASH_ALGORITHM=bcrypt
   BCRYPT_COST=12  // Higher = more secure but slower
   ```

3. **HTTPS in Production**
   ```env
   # Force HTTPS
   REMEMBER_ME_SECURE=true
   SESSION_SECURE=true
   
   # Use secure cookies
   COOKIE_SAME_SITE=strict
   COOKIE_HTTP_ONLY=true
   ```

4. **Rate Limiting**
   ```rust
   // Protect sensitive endpoints
   app.route("/login")
       .layer(RateLimitLayer::new(5, Duration::from_secs(60)));
   
   app.route("/api/*")
       .layer(RateLimitLayer::new(100, Duration::from_secs(60)));
   ```

5. **Input Validation**
   ```rust
   // Always validate user input
   use rf_validation::*;
   
   #[derive(Validate, Deserialize)]
   struct UserInput {
       #[validate(email)]
       email: String,
       
       #[validate(length(min = 8, max = 100))]
       password: String,
   }
   ```

6. **Database Security**
   ```rust
   // NEVER use raw SQL with user input
   // BAD:
   let query = format!("SELECT * FROM users WHERE email = '{}'", user_email);
   
   // GOOD: Use parameterized queries
   let user = User::find()
       .filter(user::Column::Email.eq(&user_email))
       .one(&db).await?;
   ```

7. **Secrets Management**
   ```bash
   # Use secret management tools in production
   # - AWS Secrets Manager
   # - HashiCorp Vault
   # - Kubernetes Secrets
   # - Docker Secrets
   
   # NEVER hardcode secrets
   # BAD:
   let api_key = "sk_live_1234567890abcdef";
   
   # GOOD:
   let api_key = env::var("API_KEY")?;
   ```

### For Framework Contributors

1. **Code Review**
   - All PRs require security review
   - Security-sensitive changes need 2+ approvals
   - Run security audits before merge

2. **Dependencies**
   - Regular dependency updates
   - Security advisories monitoring
   - `cargo audit` in CI/CD

3. **Testing**
   - Security test cases required
   - Penetration testing for major releases
   - Fuzzing for critical components

---

## Security Audits

### v1.0.0 Security Audit (November 2025)

**Overall Grade: B+**

**Password Security: A**
- Argon2/Bcrypt properly configured
- Secure salt generation
- Timing-safe comparison

**Token Security: A**
- JWT implementation correct
- Proper expiration handling
- Secure signature verification

**Network Security: B+**
- TLS/SSL enforced
- CORS configured
- Rate limiting implemented

**Storage Security: B+**
- Presigned URLs time-limited
- Path traversal prevented
- Access control basic (needs RBAC)

**Recommendations:**
1. Implement RBAC/Permissions (planned v1.1.0)
2. Add CSP/HSTS headers
3. Encrypt audit logs at rest
4. Add MFA support

---

## Security Tools

### Recommended Tools

1. **cargo-audit**
   ```bash
   cargo install cargo-audit
   cargo audit
   ```

2. **cargo-deny**
   ```bash
   cargo install cargo-deny
   cargo deny check
   ```

3. **RustSec Advisory Database**
   - Automatic checks in CI/CD
   - https://rustsec.org/

4. **OWASP ZAP**
   - Web application security scanner
   - https://www.zaproxy.org/

5. **SQLMap**
   - SQL injection detection
   - https://sqlmap.org/

---

## Vulnerability Disclosure Timeline

### Example Timeline (Hypothetical)

**Day 0:**
- Vulnerability reported via email
- Auto-response sent to reporter

**Day 1:**
- Security team reviews report
- Initial assessment: Severity MEDIUM
- Response sent to reporter

**Day 3:**
- Fix developed and tested
- Internal security review completed

**Day 5:**
- Patch released as v1.0.1
- Security advisory published
- Reporter credited

**Day 7:**
- Public disclosure on GitHub
- Blog post with details
- Social media announcement

---

## Security Checklist for Production

Before deploying to production, ensure:

### Infrastructure
- [ ] HTTPS enforced (TLS 1.2+)
- [ ] Firewall configured
- [ ] Redis password protected
- [ ] Database credentials rotated
- [ ] Backup strategy in place

### Application
- [ ] All secrets in environment variables (not hardcoded)
- [ ] Debug mode disabled
- [ ] Error messages sanitized (no sensitive info)
- [ ] Rate limiting enabled
- [ ] CORS properly configured

### Monitoring
- [ ] Security logging enabled
- [ ] Intrusion detection configured
- [ ] Alerting set up
- [ ] Audit logging enabled
- [ ] Log aggregation in place

### Compliance
- [ ] Data encryption at rest (if required)
- [ ] Data encryption in transit
- [ ] GDPR compliance (if applicable)
- [ ] PCI DSS compliance (if handling cards)
- [ ] Regular security audits scheduled

---

## Contact

For security-related questions:
- **Email**: security@rustforge.dev
- **GitHub**: https://github.com/Chregu12/RustForge/security
- **Discussions**: https://github.com/Chregu12/RustForge/discussions

**DO NOT** use public channels (issues, discussions) for vulnerability reports.

---

## Updates

This security policy may be updated periodically. Check back for the latest version.

**Last Updated:** November 13, 2025  
**Version:** 1.0
