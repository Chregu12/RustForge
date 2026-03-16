# RustForge - Known Issues & Bug Tracker

This file documents known issues, incomplete implementations, and technical debt
across the RustForge framework. Issues are organized by severity and crate.

---

## Already Fixed (Session: 2026-03-15)

| # | Crate | Issue | Commit |
|---|-------|-------|--------|
| 1 | `rf-web` | **CRITICAL SECURITY**: CSRF middleware only checked header existence, never validated token value against server-side store | `b3cef85` |
| 2 | `rf-web` | **CRITICAL**: Session middleware used `futures::executor::block_on()` inside async → deadlock under Tokio | `b3cef85` |
| 3 | `rf-web` | Multiple `unwrap()` panics in session middleware during session creation and cookie parsing | `b3cef85` |
| 4 | `rf-web` | `Set-Cookie` header used `insert()` instead of `append()`, silently dropping other cookies | `b3cef85` |
| 5 | `rf-jobs` | `SerializedJob` was `pub(crate)` but used in public `QueueManager` methods in batch.rs and chain.rs | latest |
| 6 | `rf-jobs` | Duplicate `clone_schedules` implementation - inherent method shadowed by trait method, unused dead code | latest |
| 7 | `rf-jobs` | `WorkerPool.queue_manager` and `WorkerPool.registry` stored but never read after construction | latest |
| 8 | `rf-jobs` | `ScheduledJob.job_factory` stored but never called - scheduler can't dispatch jobs | latest |
| 9 | `rf-orm` | Unused imports: `Func`, `SelectStatement`, `SimpleExpr`, `OrderedStatement` in query_builder.rs | latest |
| 10 | `rf-orm` | `integration-tests` feature flag referenced but not defined in Cargo.toml | latest |
| 11 | `rf-orm` | Unused variable `result` in `pool_optimizer.rs::health_check()` | latest |
| 12 | `rf-orm` | `async fn` in public trait without `Send` bound (`Model` facade) | latest |
| 13 | `rf-response` | Unused `Json` import | latest |
| 14 | `rf-mail` | Duplicate `Mailable` trait in `mailer.rs` shadowed by `mailable.rs::MailableAsync` | latest |
| 15 | `rf-web` | Unused imports: `HeaderValue`, `async_trait` in versioning.rs | latest |
| 16 | `rf-validation-derive` | `field.ident.as_ref().unwrap()` panics on tuple struct fields | latest |
| 17 | Workspace | `rf-command-events`, `rf-command-pipeline`, `rf-advanced-input`, `rf-infra`, `rf-api`, `rf-oauth`, `rf-oauth-server`, `foundry-cli` incorrectly disabled - all compile cleanly | latest |
| 18 | `rf-cache` | `MemoryCache::delete()` did not remove key from tag sets — orphaned tag references accumulated forever | latest |
| 19 | `rf-cache` | `Cache::increment()`/`decrement()` default impl always reset TTL to hardcoded 86400s; `MemoryCache` now overrides to preserve the remaining TTL of the original entry | latest |
| 20 | `rf-scheduler` | `ScheduledTask.running` flag was set to `false` immediately after `tokio::spawn()` before the spawned task completed — race condition allowed duplicate concurrent execution | latest |
| 21 | `rf-scheduler` | Overlap prevention only guarded tasks with `prevent_overlap() = true`; non-overlapping tasks were never inserted into `running_tasks` leaving them completely untracked | latest |
| 22 | `rf-broadcasting` | `AuthToken::sign()` used `std::collections::hash_map::DefaultHasher` (non-deterministic, non-cryptographic) for token signing — replaced with proper HMAC-SHA256 (RFC 2104) | latest |
| 23 | `rf-broadcasting` | `WebSocketHandler` spawned forwarding tasks on subscribe but never stored their `JoinHandle` — unsubscribing or disconnecting left orphaned tasks running; now aborts on unsubscribe/disconnect | latest |
| 24 | `rf-queue` | `RedisQueue::complete()` deleted job data but never removed the entry from the Redis processing list — processed jobs accumulated in `processing:{queue}` forever | latest |
| 25 | `rf-queue` | `RedisQueue::fail()` moved job to failed list but never removed it from processing list or deleted the original `job:{id}` key | latest |
| 26 | `rf-queue` | `RedisQueue::retry()` re-enqueued job without first removing it from the processing list — same job existed in both processing AND delayed queues simultaneously | latest |
| 27 | `rf-oauth2-server` | **SECURITY**: `Client::verify_secret()` used `==` string comparison (timing side-channel) — replaced with SHA-256 digest comparison over fixed-length outputs | latest |
| 28 | `rf-oauth` | **SECURITY**: Facebook provider sent access token in URL query string — token logged in HTTP access logs and proxies; replaced with `Authorization: Bearer` header | latest |
| 29 | `rf-encryption` | **SECURITY**: `EncryptorBuilder::build()` silently used a 32-byte zero key when no key was provided — all default-built instances were identically vulnerable; `build()` now returns `Result` and fails explicitly | latest |
| 30 | `rf-encryption` | **SECURITY**: `EncryptorBuilder::key()` fell back to treating raw input bytes as the key if base64 decoding failed — predictable low-entropy keys; fallback removed, only base64 input accepted | latest |
| 31 | `rf-ratelimit` | Tests called `redis_available()` which was never defined — would cause test compilation failure; helper function added | latest |
| 32 | `rf-validation` | `MinLengthRule` and `MaxLengthRule` used `str.len()` (byte count) instead of `str.chars().count()` (character count) — multi-byte Unicode characters (e.g. emojis) failed or passed incorrectly | latest |
| 33 | `rf-mail` | `MailgunMailer::send_via_api()` looped over recipients and called `form.insert("to", addr)` in each iteration — `HashMap::insert` overwrites the previous value so only the **last** recipient was actually sent; fixed by joining all addresses into a comma-separated string | latest |
| 34 | `rf-search` | `InMemoryEngine::search()` called `partial_cmp().unwrap()` when sorting results by score — panics if any score is NaN; replaced with `unwrap_or(Ordering::Equal)` | latest |
| 35 | `rf-search` | **SECURITY**: `PostgresSearchDriver` interpolated `T::index_name()` directly into SQL `DELETE`/`TRUNCATE`/`COUNT` via `format!()` — SQL injection if implementor returns a malicious identifier; added `validate_identifier()` that rejects anything not `[a-zA-Z0-9_]` | latest |
| 36 | `rf-forms` | **CRITICAL SECURITY**: `FormRenderer` interpolated all user-controlled values (field names, labels, values, placeholders, help text, error messages, form action/name) directly into HTML without escaping — XSS via any form field; added `html_escape()` helper that encodes `& < > " '` and applied it to all interpolation points | latest |
| 37 | `rf-forms` | `FormRenderer` emitted hardcoded `value="TOKEN_HERE"` for CSRF hidden input instead of an actual token — CSRF protection was non-functional; added `csrf_token: Option<String>` field to `Form`/`FormBuilder` so the renderer uses the real token | latest |
| 38 | `rf-pagination` | `Paginator::offset()` computed `(current_page - 1) * per_page` using wrapping arithmetic — integer overflow with large page/per_page values could produce negative offsets or wrap to unexpected values; replaced with `saturating_mul` and `saturating_add` | latest |
| 39 | `rf-graphql` | `OffsetPaginationInput::offset()` multiplied `page * per_page` as `i32` before casting to `i64` — overflows and panics in debug or wraps in release for large values; cast to `i64` before multiplying | latest |
| 40 | `rf-graphql` | `PaginatedResult::new()` accepted `per_page=0` causing division by zero in float arithmetic — `(total as f64) / (0 as f64)` produces `inf`/`NaN`, yielding `i32::MAX` or `0` for `total_pages`; clamped `per_page` to minimum of 1 | latest |
| 41 | `rf-upload` | **DATA LOSS**: `FileUpload::store()` comment says "Generate unique filename" but code used the sanitized original name directly — two uploads with the same filename silently overwrite each other; prepended nanosecond timestamp to filename | latest |
| 42 | `rf-upload` | **SECURITY**: MIME type validation used `starts_with` prefix matching — `"image/jpegscript"` would pass validation when `"image/jpeg"` was allowed; changed to exact match for specific types, prefix match only for category wildcards ending in `/` | latest |
| 43 | `rf-collections` | `for_page(0, n)` caused `usize` underflow panic via `(page - 1) * per_page` — replaced with `saturating_sub(1)` | latest |
| 44 | `rf-collections` | `slice()` panicked when `offset > items.len()` — added `offset.min(self.items.len())` bounds clamp | latest |
| 45 | `rf-collections` | `splice()` panicked on out-of-bounds `start` or `start + length` — added bounds clamping for both | latest |
| 46 | `rf-container` | `bind_with_scope()` resolved types against a brand-new empty `ServiceRegistry` instead of the actual registry — any type with dependencies panicked on resolution; fixed by cloning `self` (which shares the underlying `Arc<Mutex<…>>` map) into the closure | latest |
| 47 | `rf-telescope` | `CacheInfo::with_value()` truncated at byte index 1000 via `&val[..1000]` — panics on multi-byte UTF-8 characters when byte 1000 falls mid-character; replaced with char-boundary-aware truncation | latest |
| 48 | `rf-telescope` | **CRITICAL SECURITY**: Dashboard JavaScript rendered all dynamic content (`req.path`, `exc.message`, `query.sql`, `mail.subject`, etc.) via template literals into `.innerHTML` without escaping — stored XSS via crafted request paths, exception messages, or SQL queries; added `escapeHtml()` function applied to all dynamic interpolations | latest |
| 49 | `rf-logging` | `init_logging()` silently returned `Ok(())` when `stdout: false` and no file output configured — all log output silently discarded; now returns an error when no output target is configured | latest |
| 50 | `rf-storage` | **SECURITY**: `LocalStorage::resolve_path()` canonicalized only existing files — for new files (`put`), the symlink-based traversal check was silently skipped, allowing writes outside storage root; now canonicalizes parent directory for non-existent paths | latest |
| 51 | `rf-passport` | **SECURITY**: `revoke_token` handler compared only `user_id` — client-credentials tokens (where `user_id` is `None`) from any client could revoke any other client's tokens; added `client_id` check for client-credentials tokens | latest |
| 52 | `rf-passport` | Refresh token scope validation used `Vec::contains()` instead of `has_scope()` — wildcard scope `"*"` not honored during token refresh, preventing scope narrowing on wildcard tokens; switched to `has_scope()` which handles `"*"` | latest |
| 53 | `rf-horizon` | `ChainHandle::wait()` looped forever when a chain job failed — `is_finished()` only checked `completed >= total` but `completed` was never incremented on failure; added `chain_done` flag set when spawned task exits (success or failure) | latest |
| 54 | `rf-horizon` | Division by zero panic in jobs API handler when `per_page=0` in query string — clamped `per_page` to minimum 1 | latest |
| 55 | `rf-horizon` | `RetryStrategy::Exponential::delay()` panicked on overflow via `2_u64.pow(retry_count)` for retry counts >= 64 — replaced with `checked_pow`/`saturating_mul` and capped at 7 days | latest |
| 56 | `rf-feature-flags` | `set_percentage()`/`enable_for_users()`/`enable_for_groups()` each created a brand-new `FlagConfig`, silently erasing all other targeting rules — now fetches existing config first and merges the update | latest |
| 57 | `rf-feature-flags` | `disable()` replaced entire flag config with a fresh disabled one, destroying user/group/percentage rules — now preserves existing config and only sets `enabled = false` | latest |
| 58 | `rf-eloquent` | `truncate` accessor panicked on multi-byte UTF-8 via `&value[..length]` byte slicing — replaced with `chars().take(length)` | latest |
| 59 | `rf-eloquent` | `detect_n_plus_one()` and `stats()` caused guaranteed deadlock — `detect_n_plus_one` held `grouped` mutex then called `create_pattern` which tried to lock it again; refactored to avoid re-locking | latest |
| 60 | `rf-eloquent` | `should_eager_load()` used `>` threshold comparison while `detect_n_plus_one()` used `>=` — off-by-one at boundary meant N+1 detected but eager load not triggered; fixed to `>=` | latest |
| 61 | `rf-http-client` | Retry logic used constant `retry_config.delay` on every attempt — `backoff_multiplier` field was silently ignored; now applies exponential backoff using the multiplier | latest |
| 62 | `rf-notifications` | **SECURITY**: `MailMessage::to_html()` interpolated greeting, lines, action URL/text directly into HTML without escaping — XSS via user-controlled content in email templates; added `escape_html()` | latest |
| 63 | `rf-routing` | `parse_signed_url()` discarded all original query parameters when reconstructing the URL — any signed URL with extra query params (e.g. `?page=1`) would lose them, causing signature verification to fail | latest |
| 64 | `rf-routing` | `QueryStringBuilder::build()` did not URL-encode keys or values — special characters (`&`, `=`, spaces, etc.) in query params produced malformed/ambiguous URLs; added percent-encoding | latest |
| 65 | `rf-config` | **SECURITY**: `AuthConfig` derived `Debug` which prints `jwt_secret` in plain text to logs/errors; replaced with manual `Debug` impl that redacts the secret | latest |
| 66 | `rf-config` | **SECURITY**: `DatabaseConfig` derived `Debug` which prints database URL (including credentials) in plain text to logs/errors; replaced with manual `Debug` impl that redacts the URL | latest |
| 67 | `rf-inertia` | **SECURITY**: `into_html_response()` embedded JSON `data-page` in single-quoted attribute without escaping — single quotes in JSON values could break attribute and inject HTML/JS; added HTML entity escaping for both JSON data and root_view | latest |
| 68 | `rf-broadcasting` | **SECURITY**: `AuthToken::verify()` used `==` string comparison for HMAC signature verification — timing side-channel leaks signature bytes; replaced with `hmac::Mac::verify_slice()` for constant-time comparison | latest |
| 69 | `rf-validation` | **CRITICAL SECURITY**: `SimpleExistsRule` and `SimpleUniqueRule` interpolated `table`, `column`, and `id_column` directly into SQL query strings without validation — SQL injection via identifier names; added `validate_sql_identifier()` that rejects non-alphanumeric/underscore characters | latest |
| 70 | `rf-oauth2-server` | **CRITICAL SECURITY**: PKCE verification compared `code_challenge` directly to `code_verifier` instead of hashing the verifier with SHA-256 and base64url-encoding per RFC 7636 — PKCE completely non-functional | latest |
| 71 | `rf-oauth2-server` | **SECURITY**: `exchange_code()` only validated client secret if caller provided one — confidential clients (with a secret) could be accessed without authentication by omitting `client_secret`; now requires authentication for clients with secrets | latest |
| 72 | `rf-envoy` | **CRITICAL SECURITY**: `authorize_key()` interpolated public key into shell command with single quotes — a key containing `'` could escape the quotes and execute arbitrary commands; added proper shell escaping | latest |
| 73 | `rf-envoy` | **CRITICAL SECURITY**: `build_script()` interpolated `working_dir` into shell script without quoting — directory paths with shell metacharacters could execute arbitrary commands; now properly quoted | latest |
| 74 | `rf-admin` | Division by zero panic in `AdminList::new()` when `per_page` is 0 — calculation of `last_page` used `per_page` as divisor without validation; fixed with `.max(1)` | latest |
| 75 | `rf-helpers` | UTF-8 panic in `plural()`/`singular()` — byte-offset string slicing (`&word[..word.len()-N]`) panics on multi-byte characters; replaced with char-based operations | latest |
| 76 | `rf-livereload` | Race condition in builder methods — `watch()`, `pattern()`, `debounce_ms()`, `port()` used `tokio::spawn()` to modify config asynchronously but returned `self` immediately, so `start()` would read default config; replaced with synchronous `blocking_write()` | latest |
| 77 | `rf-macros` | Panic in `to_snake_case()` — `c.to_lowercase().next().unwrap()` panics if lowercase iterator is empty for certain Unicode characters; fixed with `.unwrap_or(c)` in 3 files | latest |
| 78 | `rf-scaffold` | UTF-8 panic in `pluralize()`/`singularize()` — byte-offset string slicing panics on multi-byte characters; replaced with char-based and `strip_suffix` operations | latest |
| 79 | `rf-stub-system` | Panic in `CaseConverter::plural()`/`singular()` — byte-offset slicing `&s[..s.len()-N]` panics on single-char inputs or multi-byte UTF-8; replaced with `strip_suffix` operations | latest |
| 80 | `rf-cashier` | Panics on invalid Stripe IDs — 20+ instances of `.parse().unwrap()` across 6 files (billable.rs, subscription.rs, invoice.rs, checkout.rs, portal.rs, payment.rs); if stored Stripe ID is corrupted/wrong format, app panics instead of returning error; replaced with `.map_err()` | latest |
| 81 | `rf-nova` | Panic in `TrendData::by_days()` — `.succ_opt().unwrap()` panics at max date boundary; replaced with match + break | latest |
| 82 | `rf-dusk` | CSS selector injection in `select()` — user-provided value interpolated directly into CSS attribute selector without escaping; special chars like `"` could break the selector; added escaping | latest |
| 83 | `rf-tinker` | **CRITICAL SECURITY**: SQL injection in `parse_db_table_call()` — table name, column names, where values, and orderBy columns from user input regex capture interpolated directly into SQL strings without validation; added `validate_sql_identifier()` and value escaping | latest |
| 84 | `rf-application` | **SECURITY**: SQL injection via column names in `create_record()` and `update_record()` — JSON object keys used as column names interpolated directly into SQL INSERT/UPDATE without validation; added identifier validation rejecting non-alphanumeric/underscore characters | latest |
| 85 | `rf-application` | Panic in `to_snake_case()` (graphql.rs) — `ch.to_lowercase().next().unwrap()` panics on edge-case Unicode characters; fixed with `.unwrap_or(ch)` | latest |
| 86 | `rf-application` | Panic in `pascal_to_snake()` (event.rs) — same `to_lowercase().next().unwrap()` panic; fixed with `.unwrap_or(ch)` | latest |
| 87 | `rf-cli-gen` | Panic in `to_snake_case()` — `c.to_lowercase().next().unwrap()` panics on edge-case Unicode; fixed with `.unwrap_or(c)` | latest |
| 88 | `foundry-cli` | Panic in `to_snake_case()` (policy.rs, provider.rs) — `ch.to_lowercase().next().unwrap()` panics on edge-case Unicode; fixed with `.unwrap_or(ch)` in both files | latest |
| 89 | `rf-tinker` | UTF-8 panic in `formatter.rs` — `&cell[..self.max_column_width - 3]` byte-offset slicing panics on multi-byte UTF-8 characters; replaced with `cell.chars().take()` | latest |
| 90 | `rf-views` | **SECURITY**: XSS in `components.rs` — alert, card, and form input components interpolated user-controlled values (`message`, `title`, `content`, `value`, `error`, `name`, `label`, etc.) directly into HTML without escaping; added `escape_html()` to all interpolated values | latest |
| 91 | `rf-mail` | **SECURITY**: XSS in `markdown.rs` — `@button`, `@table`, and `@panel` components interpolated URL, text, header, and cell values directly into HTML email without escaping; added `escape_html()` to all interpolated values in email template rendering | latest |
| 92 | `rf-nova` | Division by zero in `PaginationMeta::new()` when `per_page` is 0 — f64 division produces infinity, which casts to broken `u64` value; fixed with `.max(1)` | latest |
| 93 | `rf-pest` | NaN/infinity in `print_progress()` when `total` is 0 — division by zero in progress bar percentage calculation; added zero-guard | latest |
| 94 | `rf-infra` | Panic in `MetricAggregate::from_metrics()` — `partial_cmp().unwrap()` panics when sorting metrics containing NaN values; fixed with `.unwrap_or(Ordering::Equal)` | latest |
| 95 | `rf-horizon` | Division by zero in `update_processing_time()` — when `jobs_processed` is 0, division by zero produces NaN average; fixed with `.max(1)` | latest |
| 96 | `rf-db-facade` | Division by zero panic in `paginate()` — integer division by `per_page` when `per_page` is 0 causes panic; fixed with `.max(1)` | latest |
| 97 | `rf-orm` | Division by zero panic in `QueryBuilder::paginate()` and `FacadeQueryBuilder::paginate()` — `per_page = 0` causes integer division panic or infinity; fixed with `.max(1)` in both implementations | latest |
| 98 | `rf-passport` | **SECURITY**: Authorization bypass in token revocation — when `token.user_id` is `None` (client-credentials token), comparison `None != None` is false, allowing any client to revoke another client's tokens; fixed by checking client-credentials tokens first via `token.user_id.is_none()` branch | latest |
| 99 | `rf-search` | Out-of-bounds panic in `search()` — when `query.offset >= hits.len()`, slice indexing `hits[start..end]` panics; fixed with `start = query.offset.min(hits.len())` | latest |
| 100 | `rf-search` | **SECURITY**: SQL injection in PostgreSQL search driver — `options.sort` field interpolated directly into `ORDER BY` clause without validation; added alphanumeric/underscore validation | latest |
| 101 | `rf-collections` | Panic in `chunk(0)` and `sliding(0)` — `slice::chunks(0)` and `slice::windows(0)` panic with zero size; fixed with `.max(1)` guard | latest |
| 102 | `rf-maintenance` | **SECURITY**: Timing attack on maintenance mode secret — `verify_secret()` used `==` string comparison vulnerable to timing side-channel; replaced with constant-time byte-by-byte XOR comparison | latest |
| 103 | `rf-spark` | Panic on invalid Stripe IDs — 35+ `.parse().unwrap()` calls across `payment.rs`, `customer.rs`, `subscription.rs`, and `invoice.rs` panic if Stripe ID strings are malformed; replaced all with `.parse().map_err(...)` returning `SparkError::InvalidRequest` | latest |
| 104 | `rf-api-resources` | Division by zero in `PaginationMeta::new()` when `per_page` is 0 — f64 division produces infinity, `ceil() as u32` gives garbage; fixed with `.max(1)` | latest |
| 105 | `rf-resources` | Division by zero in `total_pages()` when `per_page` is 0 — f64 division produces infinity, `ceil() as u64` gives garbage; fixed with `.max(1)` guard | latest |
| 106 | `rf-forms` | **SECURITY**: Timing attack on CSRF token validation — `validate()` compared tokens with `==`, vulnerable to timing side-channel; replaced with constant-time XOR comparison | latest |
| 107 | `rf-sanctum` | **SECURITY**: Timing attack on SPA CSRF token verification — `verify_csrf_token()` compared tokens with `!=`, vulnerable to timing side-channel; replaced with constant-time XOR comparison | latest |
| 108 | `rf-passport` | **SECURITY**: Timing attack on OAuth client secret verification — `verify_secret()` compared hashed secrets with `==`; replaced with constant-time XOR comparison | latest |
| 109 | `rf-envoy` | **SECURITY**: Shell injection in systemd task presets — `service`, `app_dir`, `branch` parameters interpolated directly into shell commands (`sudo systemctl restart $service`); fixed with single-quote escaping | latest |
| 110 | `rf-queue` | Panic in `new_delayed()` — `Duration::from_std(delay).unwrap()` panics if delay duration is too large; replaced with proper `map_err` error propagation | latest |
| 111 | `rf-blade` | **SECURITY**: XSS in `Slot::attributes_html()` — attribute values rendered directly into HTML without escaping `"`, `<`, `>`, `&`; added HTML escaping to all attribute values | latest |
| 112 | `rf-blade` | Panic in compiler — `serde_json::Number::from_f64(*n).unwrap()` panics on NaN/Infinity float values; replaced with `.unwrap_or(Value::Null)` | latest |
| 113 | `rf-blade` | Panic in `Expr::parse()` — single-character quote string `"` or `'` causes `s[1..0]` byte index panic; added `s.len() >= 2` guard | latest |
| 114 | `rf-views` | Unicode bug in `TruncateFilter` — `text.len()` (byte count) compared against character length, causing incorrect truncation of multi-byte strings; fixed with `text.chars().count()` | latest |
| 115 | `rf-views` | Logic bug in `normalize_template_name()` — `replace('.', "/")` corrupts file extensions (e.g., `test.tera` → `test/tera.tera`); fixed by stripping extension before dot-to-slash conversion | latest |
| 116 | `rf-views` | Data loss in `ViewEngine::set_error()` — each call creates a new HashMap and overwrites all previous errors in `errors_function` and `has_error_function` via `set_errors()`; fixed by using `add_errors()` to merge instead of replace | latest |
| 117 | `rf-blade` | Silent parse error in `read_directive_args()` — unterminated parentheses (missing closing `)`) causes the lexer to silently succeed and return truncated args instead of an error; added `depth > 0` check after loop | latest |
| 118 | `rf-inertia` | Deferred props included in initial response — `build()` never filters deferred props from initial page load and never sets `deferred_props` metadata; fixed by removing deferred keys from props on non-partial requests and calling `with_deferred_props()` | latest |
| 119 | `rf-application` | **SECURITY**: Shell injection + SQL injection in `DatabaseCreateCommand` — `db_name`, `db_user`, `root_password`, `host` from user input interpolated directly into shell commands and SQL; added identifier validation and shell escaping | latest |
| 120 | `rf-auth-scaffolding` | Panic in `send_email()` — `.parse().unwrap()` on `from_email` config and `to` address panics on invalid email; replaced with `?` error propagation | latest |
| 121 | `rf-routing` | **SECURITY**: Timing attack in `SignedUrl::verify()` — URL signature compared with `!=` allowing timing side-channel; replaced with constant-time XOR comparison | latest |
| 122 | `rf-broadcast` | **SECURITY**: Timing attack in `PusherBroadcaster::verify_webhook()` — HMAC signature compared with `==` (comment incorrectly says "Constant-time comparison"); replaced with XOR comparison | latest |
| 123 | `rf-cashier` | **SECURITY**: Timing attack in Stripe webhook `verify_signature()` — HMAC signature compared with `!=`; replaced with constant-time XOR comparison | latest |
| 124 | `rf-orm` | SQL injection in `remove_migration()` — migration name interpolated directly into `DELETE FROM migrations WHERE migration = '{}'` without escaping quotes; added single-quote escaping | latest |
| 125 | `rf-search` | **SECURITY**: SQL injection in `create_fts_index()` and `drop_fts_index()` — `table` and `columns` parameters not validated before interpolation into DDL statements; added `validate_identifier()` calls | latest |
| 126 | `rf-cms` | **SECURITY**: XSS/JS injection in `EditorConfig::init_script()` — `selector` parameter embedded directly into JavaScript string literals without escaping; added JS string escaping for `\`, `'`, `\n`, `\r` | latest |
| 127 | `rf-nova` | Non-deterministic trend direction in `TrendMetric::calculate_trend()` — iterates `HashMap` values (undefined order) to determine trend; fixed by sorting entries by key before comparison | latest |
| 128 | `rf-nightwatch` | Panic in `Histogram::min()`/`max()` — `partial_cmp().unwrap()` panics when NaN values are recorded; added NaN filtering and fallback ordering | latest |
| 129 | `rf-nightwatch` | Panic in `Histogram::percentile()` — NaN causes sort panic, and floating-point index can exceed bounds; added NaN filtering and index clamping | latest |
| 130 | `rf-spark` | Silent data loss in financial amount conversion — `(amount * 100).to_string().parse::<i64>().unwrap_or(0)` fails for most decimal amounts (e.g., `"1999.00"` doesn't parse as i64), silently charging $0.00; fixed with `round_dp(0)` and proper error propagation in `payment.rs` and `invoice.rs` | latest |
| 131 | `rf-testing` | Broken base64 encoding in HTTP test client — `base64::encode()` mock returned `"base64:{input}"` placeholder instead of actual base64, producing invalid Basic Auth headers; replaced with proper base64 implementation | latest |
| 132 | `rf-socialite` | Silent empty user ID from OAuth — `user_data.id.unwrap_or_default()` silently creates users with empty ID when OAuth provider omits user ID field, causing downstream auth/identity failures; changed to return error | latest |
| 133 | `rf-echo` | Presence channel always sends empty `channel_data` — `join()` calls `authenticate()` (returns only auth token) instead of `authenticate_presence()` (returns auth + channel_data), so presence channel subscriptions never include user identity data | latest |
| 134 | `rf-search` | SQL injection in PostgreSQL search filter fields — `build_where_clause()` interpolates filter field names directly into SQL without validation; added `validate_identifier()` check for all filter field names | latest |
| 135 | `rf-health` | PingCheck uses port 80 for HTTPS URLs — after stripping `https://` prefix, always defaults to port 80 instead of 443, causing health checks against HTTPS endpoints to fail | latest |
| 136 | `rf-2fa` | Wrong error for already-used backup codes — `use_code()` returns `BackupCodeNotFound` instead of distinguishing between non-existent and already-used codes; added `BackupCodeAlreadyUsed` variant | latest |
| 137 | `rf-metrics` | Metrics handler returns HTTP 200 OK with empty body on UTF-8 encoding failure — `String::from_utf8` error silently swallowed with `unwrap_or_else` returning empty string; changed to return 500 Internal Server Error | latest |
| 138 | `rf-forms` | Date validation accepts invalid dates like Feb 31 — only checks `day <= 31` without month-specific limits; added per-month day validation with leap year handling | latest |

---

## Open Issues - High Priority

### `rf-jobs/src/scheduler.rs` - Scheduler Cannot Dispatch Jobs
**Severity**: High
**File**: `crates/rf-jobs/src/scheduler.rs:162-176`

The `Scheduler` stores `job_factory` closures in `ScheduledJob` but the `run_scheduler()` loop
only extracts `(Schedule, String)` pairs via `clone_schedules()`, discarding the factory. This means
scheduled jobs are never actually dispatched. The code explicitly logs:
```
"Job dispatching not yet implemented (needs job registry)"
```
**Fix needed**: Integrate `JobRegistry` into `Scheduler` and call `registry.execute()` when a cron
trigger fires, similar to how `Worker::execute_job_payload()` works.

---

### `rf-application/src/auth/database.rs` - User Creation Not Implemented
**Severity**: High
**File**: `crates/rf-application/src/auth/database.rs:87`

The `create_user()` method does not actually create users in the database. It attempts to retrieve
a user as a workaround, returning an error if not found. Affects all registration flows.

**Fix needed**: Implement proper SeaORM entity-based user creation using `ActiveModel`.

---

### `rf-application/src/commands/tier3/admin.rs` - Admin CRUD Stubs
**Severity**: Medium
**File**: `crates/rf-application/src/commands/tier3/admin.rs:85-116`

All CRUD operations (`list`, `get`, `create`, `update`, `delete`, `validate`) return empty/passthrough
responses. The admin panel is non-functional for real data.

---

## Open Issues - Medium Priority

### `rf-orm` - Duplicate Eager Loading Implementations
**Severity**: Medium
**Files**:
- `crates/rf-eloquent/src/eager_loading.rs`
- `crates/rf-eloquent/src/eager_loading_optimized.rs`

Two separate eager loading implementations exist with no clear guidance on which to use.
The `_optimized` variant likely supersedes the original but both are exported.

**Fix needed**: Deprecate or remove `eager_loading.rs`, promote `eager_loading_optimized.rs`
as the canonical implementation.

---

### `rf-oauth2-server/src/middleware.rs` - Token Validation Not Implemented
**Severity**: Critical (security)
**File**: `crates/rf-oauth2-server/src/middleware.rs:82-103`

The `extract_bearer_token()` middleware extracts a token from the `Authorization` header but has
a `TODO` comment explicitly stating it does **not** validate the token against the database:
```
// TODO: Validate token against database/cache
// For now, just pass it through
```
Any bearer token string (including fabricated ones) passes the middleware check.

**Fix needed**: Implement database/cache lookup for the token, verify expiry, load scopes,
and reject with `401 Unauthorized` if the token is invalid.

---

### `rf-queue/src/redis.rs` - No Job Timeout Enforcement
**Severity**: Medium
**File**: `crates/rf-queue/src/worker.rs:118-178`

`JobMetadata` has a `timeout_secs` field but `process_job()` never wraps the handler in a
`tokio::time::timeout()`. A hanging job will block a worker thread indefinitely.

**Fix needed**: Wrap handler call in `tokio::time::timeout(Duration::from_secs(metadata.timeout_secs))`.

---

### `rf-queue` - No Exponential Backoff for Connection Retries
**Severity**: Medium
**Files**: `crates/rf-queue/src/redis.rs`, `crates/rf-jobs/src/queue.rs`

Redis connection errors are immediately returned without retry logic. In production, transient
connection failures should be retried with exponential backoff before failing a job.

---

### `rf-web/src/csrf.rs` - CSRF Form Body Token Not Extracted
**Severity**: Medium
**File**: `crates/rf-web/src/csrf.rs:168-181`

The `extract_token()` method explicitly comments that form body parsing is not implemented:
```rust
// Then try to get from form data
// Note: This is simplified - in production, you'd need to properly parse the body
// while preserving it for the handler
None
```
This means form-based CSRF (e.g. `<input type="hidden" name="_token">`) is never validated,
only `X-CSRF-TOKEN` headers work.

**Fix needed**: Implement multipart/form-data and `application/x-www-form-urlencoded` body
parsing that preserves the body for downstream handlers.

---

### `rf-cache` - In-Memory Cache Has No Background Eviction
**Severity**: Medium

The in-memory cache backend has no proactive TTL expiration. Expired entries are only removed
lazily when `get()` is called on them — entries set with `set_with_ttl()` and never read again
will occupy memory forever in long-running applications.

**Fix needed**: Spawn a background cleanup task on `MemoryCache::new()` that periodically
sweeps `entries` and removes all items where `entry.is_expired()`.

---

### `rf-broadcasting/src/auth.rs` - Channel Authorization Not Enforced in WebSocket Handler
**Severity**: High (security)
**File**: `crates/rf-broadcasting/src/websocket.rs:218-222`

The `Subscribe` handler has a `TODO` comment acknowledging that auth tokens for private/presence
channels are logged but never validated. Any client can subscribe to `private-*` or
`presence-*` channels without providing a valid authorization token.

**Fix needed**: Call `authorize_channel()` from `rf-broadcasting::auth` when `channel` starts
with `"private-"` or `"presence-"`, reject the subscription with an `Error` message if authorization fails.

---

### `rf-broadcasting/src/websocket.rs` - Channel Registry Not Sharded
**Severity**: Medium

Uses a single `RwLock<HashMap>` for all WebSocket channels. Under high concurrent connection
counts this creates lock contention. Consider `DashMap` or sharding by channel prefix.

---

## Open Issues - Low Priority

### `rf-scheduler` - No Queue Integration
**Severity**: Medium
**File**: `crates/rf-scheduler/src/lib.rs:244-273`

`Scheduler::start()` runs tasks directly via `tokio::spawn()`. If the process crashes mid-run,
the task is lost with no persistence or retry. Laravel-style schedulers dispatch to a persistent
queue (like `rf-jobs`) so workers can retry on failure.

**Fix needed**: Add an optional `queue_dispatch` mode that serializes the task into a `JobPayload`
and pushes it to `rf-queue`, falling back to inline `tokio::spawn` when no queue is configured.

---

### `rf-jobs/src/scheduler.rs` - Scheduler Only Checks Once Per Minute
**Severity**: Low

The scheduler sleeps 30 seconds between checks but only dispatches if `current_minute != last_minute`,
effectively limiting precision to 1-minute granularity. Sub-minute cron expressions are silently ignored.

---

### `rf-mail/src/backends/ses.rs` - AWS SES Uses SHA-256 Instead of HMAC-SHA256
**Severity**: High
**File**: `crates/rf-mail/src/backends/ses.rs:240-275`

The `sign_request()` method constructs an AWS Signature Version 4 authorization header but
uses plain `Sha256::digest()` instead of HMAC-SHA256. The comment in the code acknowledges
the gap ("in production use proper HMAC"). Every API request will fail with AWS authentication
errors (HTTP 403).

**Fix needed**: Replace `Sha256::digest(string_to_sign)` with a proper HMAC-SHA256 derivation
using the `hmac` crate (same as used in `rf-broadcasting`), following the AWS SigV4
key derivation steps: `HMAC(HMAC(HMAC(HMAC("AWS4"+secret, date), region), service), "aws4_request")`.

---

### `rf-2fa` - No Rate Limiting on TOTP Verification
**Severity**: High (security)
**File**: `crates/rf-2fa/src/lib.rs:85-90`

The `verify()` method has no rate limiting or attempt throttling. A 6-digit TOTP code has
only 1,000,000 possible values; without brute-force protection, an attacker can enumerate
all codes within a short window.

**Fix needed**: Implement per-user attempt counting (e.g. in Redis with TTL) and lock out
after N failed attempts within the TOTP window.

---

### `rf-session-facade` - Global Session State Unusable in Web Context
**Severity**: High (architecture)
**File**: `crates/rf-session-facade/src/lib.rs:12-14`

The facade uses a process-global `Lazy<RwLock<HashMap<String, Value>>>` for all session data,
shared across every concurrent HTTP request. This means:
- Different users' requests see and overwrite each other's session data
- Flash messages affect all active users simultaneously
- No session isolation by request ID

**Fix needed**: This facade must not be used in web applications. Replace with the
properly request-scoped session from `rf-web`, or redesign the facade to accept
a request-scoped context parameter rather than using global state.

---

### `rf-oauth` vs `rf-oauth-server` vs `rf-oauth2-server` — Crate Duplication
**Severity**: Medium (maintainability)

Three overlapping OAuth crates exist:
- `rf-oauth`: OAuth2 **client** (social login providers)
- `rf-oauth-server`: Full OAuth2 authorization **server** with database repositories
- `rf-oauth2-server`: Simpler in-memory OAuth2 authorization **server** with security gaps

`rf-oauth-server` and `rf-oauth2-server` provide redundant server-side functionality.
`rf-oauth-server` is more complete and uses constant-time secret comparison.

**Fix needed**: Consolidate into one server crate or clearly differentiate their roles in documentation.

---

### Test Coverage ~0%
**Severity**: Low (for now, will become High as framework matures)

With 1,104 source files and only a handful of test modules, the framework has minimal unit test
coverage. Critical paths like ORM query building, authentication flows, and job processing need
dedicated test suites.

**Priority order for tests**:
1. `rf-orm` (query builder, relationships)
2. `rf-auth` (guards, tokens)
3. `rf-jobs` (dispatch, retry, DLQ)
4. `rf-validation` (all 50+ rules)
5. `rf-web` (middleware stack integration)

---

### `rf-orm/src/facade/model.rs` - String-Based Error Propagation
**Severity**: Low

The `Model` trait returns `Result<_, String>` throughout. This loses error type information
and makes programmatic error handling difficult. Should use a proper `ModelError` enum.

---

### Multiple Crates - Inconsistent Error Types
**Severity**: Low

Five different error systems with no unified trait:
- `rf-core::AppError`
- `rf-orm::DbError`
- `rf-cache::CacheError`
- `rf-queue::QueueError`
- `rf-broadcasting::BroadcastError`

**Fix needed**: Add `From<X> for AppError` conversions or a common `FrameworkError` trait.

---

## Disabled / Deprecated Crates

| Crate | Status | Reason |
|-------|--------|--------|
| `rf-oauth` | Re-enabled