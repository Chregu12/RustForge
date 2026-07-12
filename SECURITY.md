# Security Policy

## Threat Model

### Trust boundaries

| Boundary | Trust level |
|---|---|
| Framework code running in the process | Fully trusted |
| Environment variables / `.env` file | Trusted (operator-controlled) |
| HTTP request headers, body, cookies | **Untrusted** — validated and sanitised by the framework |
| Session data stored server-side | Trusted once authenticated; keyed by a cryptographically random ID |
| Session cookie sent by the browser | Untrusted — treated as an opaque ID that must exist in the server-side store |

### What RustForge protects against

| Threat | Mechanism | Status |
|---|---|---|
| **CSRF** | Synchronizer-token pattern (`rf_web::CsrfLayer` / `CsrfTokenStore`). Tokens are 256-bit random, single-use, expire after 2 h, and compared in constant time (`subtle::ConstantTimeEq`). | Implemented (`crates/rf-web/src/csrf.rs`). Opt-in per-router. |
| **Session fixation** | `session_scope` rejects any session ID received from a client that is not present in the server-side store. Attackers cannot plant their own ID before a victim logs in. | Implemented (`crates/rf-web/src/session_facade.rs`). |
| **Session replay / cross-client bleed** | Sessions are server-side maps keyed by a 256-bit random ID. The cookie carries only the ID; session data never travels to the client. | Implemented and integration-tested (`tests/probe-sweep/tests/session_per_client.rs`). |
| **Missing APP_KEY in production** | `ApplicationServiceProvider::boot()` returns a hard error if `APP_ENV=production` and `APP_KEY` is empty or a placeholder. The server refuses to start. | Implemented (`crates/rf-service-container/src/providers/application.rs`). |
| **Clickjacking / MIME sniffing / referrer leaks** | `rf_web::SecurityHeadersLayer` adds `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, and `Referrer-Policy: no-referrer` by default. HSTS and CSP are opt-in. | Implemented (`crates/rf-web/src/middleware/headers.rs`). |
| **Timing attacks on CSRF tokens** | Token comparison uses `subtle::ConstantTimeEq`. | Implemented (`csrf.rs:42`). |
| **DB credential logging** | `rf-orm` masks the password component of connection URLs before logging. | `crates/rf-orm/src/manager.rs:68` — `mask_password()`. |

### What RustForge does NOT protect against (residual risk)

- **CSRF via form-body tokens** — The current `CsrfLayer` reads the token from the `X-CSRF-TOKEN` header only. Form-body (`_token` field) parsing requires consuming the request body, which is not yet wired. APIs using JSON or headers are fully protected; traditional HTML form submissions must use the `X-CSRF-TOKEN` header (e.g., via a `<meta>` tag and JavaScript).
- **TLS / certificate management** — The framework does not terminate TLS. Deploy behind a TLS-terminating reverse proxy (nginx, Caddy, AWS ALB) or use `rustls` directly in the application.
- **Secure cookie in development** — Session cookies are set with `Secure` only when `APP_ENV=production` or `SESSION_SECURE=true`. Local development runs without `Secure` so plaintext `localhost` works. Set `SESSION_SECURE=true` if your staging environment uses HTTPS.
- **In-memory session store** — The default `session_scope` stores sessions in a process-local `RwLock<HashMap>`. Multi-process / multi-pod deployments will not share sessions. Use `rf-cache` (Redis) as the backing store for horizontal scaling.
- **CSRF token store is in-memory** — `CsrfTokenStore` is also in-process memory. Multi-process deployments need a shared store (Redis). This is the same constraint as the session store.
- **Input validation depth** — `rf-validation` covers common rules (required, min/max, email, regex). Domain-specific business rules (e.g., SQL injection beyond parameterised queries) are the application's responsibility.
- **Supply-chain / dependency vulnerabilities** — Run `cargo deny check advisories` in CI. The workspace includes `deny.toml` for this purpose.

## Security-relevant defaults

| Setting | Default | Override |
|---|---|---|
| Session cookie `HttpOnly` | `true` | Not configurable (always on) |
| Session cookie `SameSite` | `Lax` | Not currently configurable in `session_scope` |
| Session cookie `Secure` | `false` in dev, `true` when `APP_ENV=production` | `SESSION_SECURE=true` env var |
| CSRF token lifetime | 2 hours | `CsrfConfig::lifetime_hours(n)` |
| APP_KEY validation at boot | Warn in dev, **error** in production | n/a |
| Security response headers | `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy` (opt-in layer) | `SecurityHeadersConfig` builder |
| HSTS | Off | `SecurityHeadersConfig::hsts(HstsConfig::default())` |
| CSP | Off | `SecurityHeadersConfig::content_security_policy("...")` |

## Generating a secure APP_KEY

```sh
openssl rand -base64 32
```

Copy the output and prefix it with `base64:` in `.env`:

```env
APP_KEY=base64:<output-of-command-above>
```

Never commit `.env` to version control.

## Reporting a vulnerability

If you discover a security vulnerability in RustForge, **do not open a public GitHub issue**.

**Please use GitHub's private vulnerability reporting:** navigate to the repository's **Security** tab and click **"Report a vulnerability"** to open a GitHub Security Advisory. This keeps disclosure private until a patch is ready.

Please include:
- A description of the vulnerability and its impact
- Steps to reproduce or a proof-of-concept
- The version / commit hash affected

We aim to acknowledge within 72 hours and provide a fix or mitigation timeline within 14 days.

Public disclosure is coordinated after a patch is available (responsible disclosure).
