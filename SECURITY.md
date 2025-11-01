# Security Policy

## Reporting Security Issues

**Please do not publicly disclose security issues.**

If you discover a security vulnerability in RustForge, please report it responsibly by:

1. **Email**: Contact the maintainers with a detailed description
2. **GitHub Security Advisory**: Use the [Private vulnerability reporting](https://github.com/Chregu12/RustForge/security/advisories) feature

Include the following information:
- Description of the vulnerability
- Steps to reproduce (if applicable)
- Potential impact
- Suggested fix (if you have one)

We will acknowledge your report within 48 hours and work with you to resolve the issue before public disclosure.

## Security Best Practices

When using RustForge, please follow these security best practices:

### Environment Variables
- Never commit `.env` files containing secrets
- Use `.env.example` for configuration templates
- Rotate credentials regularly

### Database
- Use strong passwords for database connections
- Limit database user permissions to necessary tables
- Enable SSL/TLS for remote database connections
- Regular backups of critical data

### API Security
- Validate all user input
- Use HTTPS in production
- Implement rate limiting
- Use proper authentication and authorization

### Dependencies
- Keep Rust and dependencies updated: `cargo update`
- Review dependency changes during updates
- Monitor for security advisories

### Secrets Management
- Never hardcode secrets in code
- Use environment variables or secret management systems
- Rotate keys and tokens regularly
- Limit secret access to necessary services

## Supported Versions

| Version | Status | Security Updates |
|---------|--------|------------------|
| 0.1.x   | Latest | ✅ Yes |

## Known Security Considerations

### SQL Injection Protection
- RustForge uses Sea-ORM which provides parameterized queries
- Tinker REPL includes basic SQL injection prevention through escaping
- Always use parameterized queries in custom code

### Authentication
- RustForge currently does not include built-in authentication
- Implement your own authentication layer or use third-party solutions
- Use HTTPS in production to protect credentials in transit

## Dependency Security

RustForge uses the following major dependencies:
- **Tokio** - Async runtime (trusted by major projects)
- **Sea-ORM** - Database ORM (well-maintained and reviewed)
- **Axum** - Web framework (part of Tokio ecosystem)
- **Serde** - Serialization (industry standard)

All dependencies are carefully selected and monitored for security issues.

## Vulnerability Disclosure

Once a security issue is resolved, we will:
1. Release a patched version
2. Publish a security advisory
3. Credit the reporter (with permission)
4. Document the issue in CHANGELOG.md

## Questions?

For security-related questions or clarifications, please email the maintainers or use the [GitHub Security Advisory](https://github.com/Chregu12/RustForge/security/advisories) feature.

---

Thank you for helping keep RustForge secure! 🔒
