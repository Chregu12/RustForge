# RustForge — Extension Extraction Plan (Cycle 19)

> **Status:** Phase 1 EXECUTED (cycle 19) + Phase 2 EXECUTED (cycle 21) + Phase 3 EXECUTED (cycle 24).
>
> - **Phase 1 (cycle 19):** 8 experimental crates moved to `extensions/`. Build
>   0-warnings green, `check-tiers` scans `crates/` + `extensions/`, no dangling refs.
>
> - **Phase 2 (cycle 21):** All 79 remaining non-stable crates (70 beta + 9 stub)
>   moved from `crates/` to `extensions/` in one scripted pass. `crates/` now contains
>   exactly the 34 stable-core crates. `extensions/` holds all 87 non-stable crates.
>   `RUSTFLAGS=-Dwarnings cargo check --workspace` exits 0 (0 warnings).
>   `cargo check --workspace --all-features` exits 0. `bash scripts/check-tiers.sh`
>   passes 121/121. Default build (`cargo check`, no --workspace) builds only the
>   stable core + examples/tests — no extension crates are directly in `default-members`.
>
> - **Phase 3 (cycle 24):** Umbrella split — `crates/rf` and `crates/rustforge` now
>   depend on the STABLE CORE ONLY. 11 Category-A crates (8 facade shims + rf-collections
>   + rf-errors + rf-view) moved from `extensions/` to `crates/`. All 15+ Category-B
>   crates (Blade, Inertia, Passport, Cashier, MCP, Nightwatch, SSE, upload, api-resources,
>   authorization, testing, pest, helpers, nova, horizon) removed from `rf`'s deps and
>   re-exposed via the new `extensions/rf-full` opt-in umbrella. Version bumped to
>   1.0.0-rc.3. `cargo tree -p rf -e no-dev | grep -c extensions` = 0.
>   `RUSTFLAGS=-Dwarnings cargo check --workspace --all-features` exits 0.
>   `bash scripts/check-tiers.sh` passes 122/122.
>
> - **Phase 4 remains:** (maintainer's one-way-door call) optional separate repo after
>   crates.io publication.
>
> **Problem statement:** The Scope/Maintainability dimension scored 4/10 in the
> independent review. Verdict: "a smaller RustForge with ~20 very good components
> would be far more convincing than 127 crates of varying maturity." Cycle 13
> removed 6 redundant duplicates; Cycle 18 reframed the narrative. This plan is the
> structural answer: how to make "34-crate stable core + optional extensions" real in
> code, not just in documentation.
>
> **Scope of original plan document:** Write and commit this plan. Phase 1 execution
> in cycle 19; Phase 2 execution in cycle 21.

---

## 1. The Closed-Core Proof

### 1a. The 34 Stable Crates

`bash scripts/check-tiers.sh` confirms: **stable=34, beta=70, experimental=8, stub=9**
(total: 121 crate directories under `crates/`).

The 34 stable crates are:

| # | Crate | Role |
|---|-------|------|
| 1 | `rf-core` | AppError/AppResult/RequestContext — foundation |
| 2 | `rf-web` | axum 0.8 routing/middleware stack |
| 3 | `rf-request` | Task-local request globals |
| 4 | `rf-response` | JSON/HTML response types |
| 5 | `rf-macros` | Model!/Route!/validate!/get!/post! proc-macros |
| 6 | `rf-model-macro` | Model derive proc-macro (used by rf-orm, rf-macros) |
| 7 | `rf-validation` | validate! DSL + ValidationErrors |
| 8 | `rf-validation-derive` | #[derive(Validate)] proc-macro |
| 9 | `rf-orm` | SeaORM ORM + QueryBuilder + DB facade |
| 10 | `rf-eloquent` | Eloquent-style ORM macros on top of rf-orm |
| 11 | `rf-auth` | Auth facade + require_auth middleware |
| 12 | `rf-sanctum` | Transient API token auth |
| 13 | `rf-cache` | Cache facade (Memory + Redis) |
| 14 | `rf-queue` | MemoryQueue + Worker |
| 15 | `rf-jobs` | Redis-backed WorkerPool |
| 16 | `rf-mail` | Mail facade (SMTP + FileMailer) |
| 17 | `rf-storage` | Storage facade (Local + S3) |
| 18 | `rf-events` | Type-keyed sync event bus |
| 19 | `rf-broadcast` | WebSocket broadcast / room management |
| 20 | `rf-ratelimit` | Per-client IP rate-limit layer |
| 21 | `rf-i18n` | AcceptLanguage + CLDR plural rules |
| 22 | `rf-health` | Health checker (fail-closed) |
| 23 | `rf-logging` | Trace/span IDs via tracing |
| 24 | `rf-metrics` | Prometheus registry |
| 25 | `rf-notifications` | Multi-channel Notifier |
| 26 | `rf-async-bridge` | Block-on-safe async/sync bridge |
| 27 | `rf-global-helpers` | Hash, csrf_token, redirect, back() |
| 28 | `rf-facades` | Consolidated facade re-exports |
| 29 | `rf-config` | AppConfig::from_env + Config facade |
| 30 | `rf-routing` | Routing facade (get/post/…/resource/Route) |
| 31 | `rf` | Simplified-import umbrella crate |
| 32 | `rustforge` | Single-import umbrella crate |
| 33 | `forge-cli` | Forge CLI (make:model/controller/migration) |
| 34 | `foundry-cli` | Foundry CLI (legacy scaffolding) |

### 1b. The Closed-Set Claim — Honest Audit

The task context states "the stable-core library crates form a CLOSED dependency set."
This is **approximately true** but not perfectly so. A grep-based audit of
`[dependencies]` in each stable crate's `Cargo.toml` reveals:

**27 stable library crates with no direct cross-tier rf- dependencies (the true closed set):**

```
rf-async-bridge, rf-auth, rf-broadcast, rf-cache, rf-config, rf-core,
rf-events, rf-facades, rf-global-helpers, rf-health, rf-i18n, rf-jobs,
rf-logging, rf-macros, rf-mail*, rf-metrics, rf-model-macro,
rf-notifications, rf-orm, rf-queue, rf-ratelimit, rf-request,
rf-routing, rf-sanctum, rf-validation, rf-validation-derive, rf-web
```

> `rf-mail` depends on `rf-view` (beta) only via an **optional feature**
> (`features = ["view"]`); the default build is clean.

**3 stable library crates with non-optional direct dependencies on beta crates (bugs, not by-design):**

| Stable crate | Beta dep | Nature |
|---|---|---|
| `rf-response` | `rf-view` (beta) | Non-optional; should be optional or view pulled stable |
| `rf-eloquent` | `rf-encryption` (beta) | Non-optional; encryption capability should be optional |
| `rf-storage` | `rf-plugins` (beta) | Non-optional; plugin extension hook should be optional |

These three are **bugs in the tier assignments or in the dependency wiring**. They
must be resolved before the extraction boundary is meaningful:

- Option A: move `rf-view`, `rf-encryption`, `rf-plugins` to stable.
- Option B: make the dependencies optional features in the three stable crates.

**4 stable crates with cross-tier dependencies by design (not bugs):**

| Crate | Cross-tier deps | Why |
|---|---|---|
| `rf` | 17+ beta crates | Umbrella "import everything" — by design |
| `rustforge` | `rf-nova` (experimental), `rf-horizon` (beta) | Umbrella — by design |
| `forge-cli` | `rf-cli-gen` (beta), `rf-deploy` (beta) | CLI tool builds on beta generators |
| `foundry-cli` | `rf-api`, `rf-application`, `rf-console`, `rf-infra`, `rf-interactive`, `rf-plugins` (all beta) | Legacy CLI |

The umbrella problem is addressed in Section 2. The CLI tools should either be
reclassified as `beta` (they depend on beta features) or their beta deps should be
promoted to stable.

**Summary:** The closed set holds for 27 of 34 stable crates. The remaining 7 have
cross-tier deps — 4 by design (umbrellas, CLIs) and 3 by bug (rf-response,
rf-eloquent, rf-storage). Extracting extensions without fixing the 3 bugs first
would leave the stable core with dangling dependencies on the extension layer.

### 1c. The 78 Extension Crates

**Beta crates (70) — real implementations, no 1.0 SemVer promise:**

```
rf-2fa, rf-admin, rf-advanced-input, rf-ai, rf-api, rf-api-resources,
rf-application, rf-assets, rf-audit, rf-auth-scaffolding, rf-authorization,
rf-blade, rf-cashier, rf-cli-gen, rf-collections, rf-command-events,
rf-command-executor, rf-command-pipeline, rf-console, rf-container,
rf-deploy, rf-domain, rf-dusk, rf-echo, rf-encryption, rf-env, rf-envoy,
rf-errors, rf-export, rf-feature-flags, rf-forms, rf-graphql, rf-helpers,
rf-horizon, rf-http-client, rf-inertia, rf-infra, rf-interactive,
rf-maintenance, rf-mcp, rf-nightwatch, rf-observability, rf-package-dev,
rf-pagination, rf-passport, rf-pest, rf-plugins, rf-providers, rf-requests,
rf-resources, rf-sail, rf-scaffold, rf-scheduler, rf-search, rf-seeder,
rf-service-container, rf-signal-handler, rf-socialite, rf-soft-deletes,
rf-spark, rf-sse, rf-stub-system, rf-tenancy, rf-testing, rf-tinker-enhanced,
rf-upload, rf-vector, rf-verbosity, rf-view, rf-views
```

**Experimental crates (8) — excluded from default-members, no SemVer guarantees:**

```
rf-breeze, rf-cms, rf-livereload, rf-nova, rf-nova-macros,
rf-swagger, rf-telescope, rf-vite
```

**Stub crates (9) — non-workspace-member dead-code directories (superseded Phase 20):**

```
rf-auth-facade, rf-cache-facade, rf-db-facade, rf-event-facade,
rf-mail-facade, rf-passport-facade, rf-route-facade, rf-sanctum-facade,
rf-storage-facade
```

> Stubs are not workspace members and thus not compiled. They should be deleted in a
> cleanup pass but are kept for backward compatibility with any downstream
> `path = ...` references. They are NOT candidates for the extensions layer —
> they are candidates for deletion.

---

## 2. Umbrella Handling

### 2a. The Problem

Both `rf` (stable) and `rustforge` (stable) are designed as "single import for
everything." They pull in the full framework including beta and even experimental
crates. This means:

- `rf` currently depends on 17+ beta crates (rf-authorization, rf-blade, rf-cashier,
  rf-collections, rf-errors, rf-helpers, rf-inertia, rf-mcp, rf-nightwatch,
  rf-pagination, rf-passport, rf-pest, rf-requests, rf-sse, rf-testing, rf-upload,
  rf-view) plus 7 stub crates.
- `rustforge` currently depends on `rf-nova` (experimental) and `rf-horizon` (beta).

This violates the intent of the `stable` tier. More critically: if extensions are
moved to a separate location (or separate repo), both umbrella crates break because
their `path = "../rf-xxx"` references would need updating.

### 2b. The Fix: Two-Umbrella Design

**`rf` is split into two crates:**

| Crate | Tier | Re-exports | Path |
|---|---|---|---|
| `rf` (redesigned) | stable | Core stable library crates only (crates 1–30 above, no beta, no experimental) | `crates/rf` |
| `rf-full` (new) | beta | Re-exports `rf` + all extension crates | `extensions/rf-full` or `crates/rf-full` |

**`rustforge` is handled the same way:**

| Crate | Tier | Re-exports |
|---|---|---|
| `rustforge` (redesigned) | stable | Core only; drop rf-nova and rf-horizon |
| `rustforge-full` (new) | beta | Re-exports `rustforge` + all extensions |

### 2c. API Impact (Honest — This Is a Breaking Change)

Users who currently `use rf::prelude::*` and rely on beta items pulled in by `rf`
will find those items missing from `rf` after this change. Concretely:

Items currently in `rf` that would be removed from the stable `rf`:

```
rf_authorization::{Gate, Policy, …}    (rf-authorization, beta)
rf_blade::{BladeEngine, …}             (rf-blade, beta)
rf_cashier::{Subscription, …}          (rf-cashier, beta)
rf_collections::{Collection, collect}  (rf-collections, beta)
rf_errors::{RustForgeError, …}         (rf-errors, beta)
rf_helpers::{Str, Arr, …}              (rf-helpers, beta)
rf_inertia::{InertiaResponse, …}       (rf-inertia, beta)
rf_mcp::{McpServer, …}                 (rf-mcp, beta)
rf_nightwatch::{Monitor, …}            (rf-nightwatch, beta)
rf_pagination::{Paginator, …}          (rf-pagination, beta)
rf_passport::{OAuthServer, …}          (rf-passport, beta)
rf_pest::{PestParser, …}               (rf-pest, beta)
rf_requests::{FormRequest, …}          (rf-requests, beta)
rf_sse::{SseEmitter, …}                (rf-sse, beta)
rf_testing::{TestRequest, …}           (rf-testing, beta)
rf_upload::{UploadedFile, …}           (rf-upload, beta)
rf_view::{ViewEngine, …}               (rf-view, beta)
```

**Migration path for users:** Replace `rf` dependency with `rf-full` in `Cargo.toml`.
The `rf-full` crate re-exports everything `rf` currently does, so no code changes are
needed — only the `Cargo.toml` entry changes.

> `rf-full` should be published alongside `rf` on crates.io, with the crate
> description explicitly stating it is the "batteries-included" variant.

**Items that stay in the stable `rf`:**

The entire surface documented in `docs/STABLE_CORE.md` remains available:
routing, request/response, validation, ORM (rf-orm + rf-eloquent), auth (rf-auth +
rf-sanctum), cache, queue, mail, storage, events, broadcast, i18n, health, logging,
metrics, notifications, config, helpers (rf-global-helpers only), facades, macros.

This is not a trivial stable surface — it covers every capability a production API
needs. The beta extensions are genuinely optional enhancements.

---

## 3. Options with Honest Tradeoffs

### Option A: In-Repo Split (move to `extensions/` within same workspace)

**What changes:** A new top-level directory `extensions/` is created. The 78
extension crates (70 beta + 8 experimental) are moved from `crates/xxx` to
`extensions/xxx`. Workspace `members` entries are updated from `"crates/xxx"` to
`"extensions/xxx"`. Path dependencies within the extensions that reference
`path = "../rf-xxx"` are updated to `path = "../../crates/rf-xxx"` (pointing to
stable core) or `path = "../rf-yyy"` (pointing to another extension).

**check-tiers.sh** must be updated to also scan `extensions/*/Cargo.toml`.

**Tradeoffs:**

| Benefit | Honest caveat |
|---|---|
| Visible structural separation: `crates/` = stable core, `extensions/` = optional | This is primarily a perception improvement for humans browsing the repo. `cargo check --workspace` still compiles all 121 crates. |
| Reversible (same repo, same workspace) | No actual reduction in CI compile time, CI job count, or maintenance burden — all crates still build together. |
| No separate GitHub repo, no separate publishing pipeline, no separate versioning | Does NOT address the core review criticism: bus-factor 1 maintaining 121 crates regardless of directory layout. |
| `check-tiers.sh` covers the new location with a two-line change | Extensions are still in `default-members` by default (or we explicitly remove them, which partially resolves the CI load issue). |
| `default-members` can already exclude experimental crates | If experimental is also removed from `members`, they no longer compile with `cargo check --workspace` (bitrot risk). |

**What this DOES NOT do:**
- Does not reduce the number of crates the CI must compile.
- Does not reduce the maintainer's surface area.
- Does not give extensions their own release cadence.
- Does not reduce the compile time for users who add `rf` as a dependency.
- The review's actual concern (bus-factor 1, 121 crates, Scope=4) is not materially
  addressed by directory rearrangement alone.

**This is mostly a perception improvement, not a maintenance-surface reduction.**

### Option B: Separate Repository (companion `rustforge-extensions` repo or per-extension repos)

**What changes:** Extension crates leave this repository entirely. They live in a
separate GitHub repository (e.g., `RustForge/rustforge-extensions`) with their own
`Cargo.toml` workspace, their own CI, and their own versioning. They reference the
core via version dependencies (`rf-core = "1.0"`) rather than path dependencies.

This requires publishing the stable core to crates.io first (currently not done).

**Tradeoffs:**

| Benefit | Honest caveat |
|---|---|
| Genuinely reduces core repo compile time: `cargo check --workspace` on the core repo builds only 34 crates (or ~27 library crates) | The core is NOT currently published to crates.io; this step is a prerequisite and a significant effort in itself. |
| Core repo CI is lean: fast compile, tight failure scope | Two-repo maintenance is harder to keep synchronized when extensions depend on in-progress core changes — you lose the monorepo's atomic cross-crate refactoring. |
| Extensions can release independently, fix bugs without core releases | Version pinning between repos becomes a coordination problem. Today a path dep change is one commit; cross-repo it becomes a synchronized release. |
| Gives the impression of a "maintained core + ecosystem" model (like axum/tower relationship) | The ecosystem framing only works if other developers actually maintain extensions; with bus-factor 1, two repos means one developer maintaining two repos instead of one. |
| Irreversible in practice (external dependents, published crates, separate Git history) | This is a one-way door. Merging back would be disruptive. |

**What this DOES do** (that Option A does not):
- Genuinely reduces the core's CI compile surface.
- Genuinely gives extensions their own release cadence once published to crates.io.
- Signals architectural maturity: "stable core with external extension ecosystem."

**What this DOES NOT do:**
- Does not reduce bus-factor (still one maintainer of both repos).
- Does not magically improve extension quality.
- Does not make extensions easier to test (they still need the same real-service CI).
- The review's Scope=4 score was primarily about "121 crates of varying maturity,"
  not specifically about repo count. A separate repo with 70 beta crates still has
  the same maturity problem.

**This option delivers the review's real goal (reduced maintenance surface) but is
irreversible and requires crates.io publication first.**

### Option C: Hybrid (in-repo split now, separate repo later)

**Phase 1:** Move extensions to `extensions/` in this workspace (Option A mechanics).
This is the reversible exploration step. It validates the dependency boundaries,
proves the `check-tiers.sh` gate works in the new layout, and gives a structural
foundation for the next step.

**Phase 2 (maintainer's decision, after crates.io publication):** Promote the
`extensions/` directory contents to a separate `rustforge-extensions` repo. Because
the path-dep boundaries are already clean from Phase 1, the migration is
`s/path = "../../crates/rf-xxx"/version = "1.x"/g` rather than untangling a
deeply mixed workspace.

**This is the recommended approach.** Option A alone is mostly perception.
Option B without Option A first is risky (untested boundary). Option C lets us do
Option A's clean-up work in-repo, validate it, then decide on Option B with evidence.

---

## 4. Recommendation + Mechanics

### 4a. Recommendation

**Adopt Option C: In-repo split now (this plan's pilot), separate-repo decision
deferred until after crates.io publication.**

Rationale:
1. The review's #1 criticism (Scope=4) is about perception AND maintenance surface.
   The perception dimension is addressable now by directory reorganization.
2. The maintenance-surface dimension requires crates.io publication first so that
   extensions can reference the core via version dep, not path dep.
3. Rushing to a separate repo before publication creates a fragile unpublished
   two-repo workspace — harder to maintain, not easier.
4. Option A (in-repo) is reversible. Option B is not.
5. The phased approach lets the maintainer see what the clean boundary actually
   looks like in practice before committing to the irreversible step.

### 4b. Prerequisites Before Phase 1 Can Begin

Three stable crates have non-optional beta dependencies (Section 1b bugs). These
must be resolved first; otherwise the "stable crates depend only on each other"
invariant is false even after the structural move:

1. **`rf-response` → `rf-view` (beta):** Make `rf-view` optional in `rf-response`
   (feature-gate it), or promote `rf-view` to stable. The simpler fix: promote
   `rf-view` to stable (it is a canonical view crate) and deprecate `rf-views`.

2. **`rf-eloquent` → `rf-encryption` (beta):** Make the encryption dependency
   optional (feature-gate it). Eloquent-style ORM should not unconditionally pull
   in AES-256-GCM encryption.

3. **`rf-storage` → `rf-plugins` (beta):** Make the plugin hook optional. Storage
   should not unconditionally pull in the undocumented plugin system.

Also decide on the CLI tools: `forge-cli` and `foundry-cli` both depend on beta
crates. Either reclassify them as `beta` (honest) or fix the deps.

### 4c. Exact Mechanics (Phased)

#### Phase 1 — Move Experimental Crates (Cycle 20 pilot)

Target: the 8 experimental crates (the smallest and highest-risk batch — no stable
crate depends on them, and they are already excluded from `default-members`).

Steps:
```
mkdir -p extensions/
git mv crates/rf-breeze extensions/rf-breeze
git mv crates/rf-cms extensions/rf-cms
git mv crates/rf-livereload extensions/rf-livereload
git mv crates/rf-nova extensions/rf-nova
git mv crates/rf-nova-macros extensions/rf-nova-macros
git mv crates/rf-swagger extensions/rf-swagger
git mv crates/rf-telescope extensions/rf-telescope
git mv crates/rf-vite extensions/rf-vite
```

For each moved crate's `Cargo.toml`, update any `path = "../rf-xxx"` to
`path = "../../crates/rf-xxx"`.

Update `Cargo.toml` (workspace root):
- Change `"crates/rf-xxx"` → `"extensions/rf-xxx"` in both `members` and
  `default-members` (experimental are already excluded from `default-members`).

Update `scripts/check-tiers.sh`:
```bash
# Scan both crates/ and extensions/
for dir in "$CRATES_DIR" "$EXTENSIONS_DIR"; do
  for cargo_toml in "$dir"/*/Cargo.toml; do
    ...
  done
done
```

Validate:
```bash
RUSTFLAGS="-Dwarnings" cargo check --workspace   # must exit 0
bash scripts/check-tiers.sh                       # must exit 0
bash scripts/reference-app-smoke.sh               # must exit 0
```

CI yaml: by-name jobs (`cargo build -p rf-nova`) are unaffected by directory
moves — they resolve by package name, not by path.

**Note about `rustforge`:** The `rustforge` umbrella currently depends on `rf-nova`
(experimental). After Phase 1, `path = "../rf-nova"` becomes
`path = "../../extensions/rf-nova"`. This path must be updated, or `rustforge`
must drop `rf-nova` as a dependency (preferred — experimental crates should not be
in a "stable" umbrella).

#### Phase 2 — Move Beta Crates in Batches (Cycles 21–23)

After Phase 1 validates the mechanics and prerequisites are fixed:

**Batch 2a (infrastructure/tooling beta crates, ~20 crates):**
rf-admin, rf-audit, rf-blade, rf-cli-gen, rf-deploy, rf-env, rf-errors,
rf-export, rf-feature-flags, rf-forms, rf-helpers, rf-horizon, rf-http-client,
rf-maintenance, rf-scaffold, rf-search, rf-seeder, rf-testing, rf-views

**Batch 2b (auth/domain extension beta crates, ~20 crates):**
rf-2fa, rf-authorization, rf-auth-scaffolding, rf-cashier, rf-dusk, rf-echo,
rf-encryption, rf-envoy, rf-graphql, rf-inertia, rf-mcp, rf-nightwatch,
rf-pagination, rf-passport, rf-pest, rf-socialite, rf-tenancy, rf-upload, rf-vector

**Batch 2c (DDD and CLI beta crates, ~30 crates):**
rf-advanced-input, rf-ai, rf-api, rf-api-resources, rf-application,
rf-assets, rf-collections, rf-command-events, rf-command-executor,
rf-command-pipeline, rf-console, rf-container, rf-domain, rf-echo, rf-envoy,
rf-infra, rf-interactive, rf-observability, rf-package-dev, rf-plugins,
rf-providers, rf-requests, rf-resources, rf-sail, rf-scheduler, rf-service-container,
rf-signal-handler, rf-soft-deletes, rf-spark, rf-sse, rf-stub-system,
rf-tinker-enhanced, rf-verbosity, rf-view

After all beta crates are moved: `crates/` contains the 27 stable library crates
plus `rf`, `rustforge`, `forge-cli`, `foundry-cli`, plus 9 stub directories
(candidates for deletion).

#### Phase 3 — Umbrella Split (Cycle 24)

After beta crates are in `extensions/`:

1. **Strip `rf` down to stable-core-only.** Remove all beta deps from
   `crates/rf/Cargo.toml`. The stable surface (STABLE_CORE.md) remains intact.

2. **Create `extensions/rf-full/Cargo.toml`.** New crate, tier `beta`, that
   re-exports `rf` + all extension crates. Users who need the full set change their
   `Cargo.toml` from `rf = "1.x"` to `rf-full = "1.x"`.

3. **Strip `rustforge` down to stable-core-only.** Remove `rf-nova` and `rf-horizon`.
   Create `extensions/rustforge-full` with the same logic.

4. Update `docs/STABLE_CORE.md` and `docs/TIERS.md` to reflect the final state.

#### Phase 4 — Separate Repo (Maintainer's Decision, Post crates.io)

After crates.io publication of the stable core:

1. Create `rustforge-extensions` GitHub repository.
2. Move `extensions/` contents there.
3. Update all `path = "../../crates/rf-xxx"` references to
   `version = "~1.x"` (pinned minor).
4. Set up separate CI in the extensions repo.
5. Archive the `extensions/` directory in this repo (or remove it and add a
   README pointing to the companion repo).

**This step is the maintainer's decision and requires crates.io publication first.**
Do NOT execute Phase 4 until:
- All stable core crates are published to crates.io.
- At least one minor release cycle has validated the stable API contract.
- The extensions repo has a separate CI setup ready.

---

## 5. Risks and Reversibility

### 5a. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Path dependency rewrites introduce subtle errors | Medium | After each batch move, run `cargo check --workspace` + all CI jobs before committing. The CI jobs are by-name so they are robust to path changes. |
| `rustforge` umbrella breaks if `rf-nova` is removed | Low | `rf-nova` is experimental; its removal from `rustforge` is desirable. No stable example or CI job depends on the `rustforge → rf-nova` path. |
| `check-tiers.sh` misses crates in `extensions/` | Low | The script update is a two-line change (add `extensions/` to the scan loop). CI gate catches any miss immediately. |
| Beta crates moved to `extensions/` that have inter-dependencies need multi-level path rewrites | Medium | Phase 2 batches are ordered so that lower-level crates (infrastructure) move before higher-level crates (application, DDD). Each batch is validated end-to-end before the next begins. |
| Stable crates with beta deps (rf-response, rf-eloquent, rf-storage) continue to violate the invariant if not fixed first | High | These MUST be fixed before Phase 2 begins. If not fixed, the stable core's Cargo.toml contains `extensions/` paths, defeating the purpose of the separation. |
| `rf-full` (new umbrella) is a breaking change for current `rf` users | Medium | The migration is a single `Cargo.toml` line change. A deprecation warning in `rf`'s `lib.rs` with a doc comment pointing to `rf-full` gives advance notice. CHANGELOG entry for the minor version introduces this. |
| The CLI tools (forge-cli, foundry-cli) depend on beta crates; if beta crates move, CLI `path` deps break | High | Fix CLI deps before Phase 2: either move CLI tools to `extensions/` (since they depend on beta), or update their path deps as part of each batch. Recommend moving CLIs to `extensions/` since they are development tools, not core library crates. |
| Separate repo (Phase 4) has never-resolved version conflicts between core releases and extension compatibility | High | This is the known cost of breaking a monorepo. Mitigate with tight `~1.x` minor pinning and a compatibility matrix in the extensions repo's README. This is why Phase 4 is deferred until post-publication validation. |

### 5b. Reversibility

| Phase | Reversible? | How |
|---|---|---|
| Phase 1 (move experimental) | YES | `git mv extensions/rf-xxx crates/rf-xxx`, revert Cargo.toml members entries, one commit |
| Phase 2 (move beta batches) | YES (while in-repo) | Same as Phase 1 — a set of `git mv` reversals |
| Phase 3 (umbrella split) | PARTIALLY — `rf`→`rf-full` is a semver-breaking change for users once published; in-repo it is reversible before publication | In-repo: revert Cargo.toml; post-publication: must use a major version bump |
| Phase 4 (separate repo) | NO — separate Git history, published crates, external dependents | Only "reversal" is vendoring extensions back into this repo, which is not a true reversal |

**The in-repo phases (1–3) are collectively reversible via `git revert`. Phase 4 is
the one-way door and should only be taken with the maintainer's explicit decision
after validating Phases 1–3 and after crates.io publication.**

### 5c. What the Extraction Does NOT Fix

Be honest about what this plan cannot deliver:

1. **Bus-factor 1 remains.** Directory structure does not create external contributors.
2. **Beta crate quality is unchanged.** Moving 70 beta crates to `extensions/` does
   not improve their test coverage, fix their TODO macros, or validate their APIs.
3. **Scope/Maintainability score improvement is bounded.** The review's Scope=4 is
   partly about the 121-crate surface and partly about "no external contributors."
   The structural separation addresses the first dimension but not the second.
4. **The stable core still has 27+ real crates.** That is not ~20 crates. The
   reviewer's "~20 very good components" target may require further pruning within
   the stable tier — for example, merging rf-queue + rf-jobs, or rf-logging +
   rf-metrics + rf-health into fewer crates. This plan does not address that.
5. **Compile time for the workspace is unchanged** until Phase 4 (separate repo).
   A user who adds `rf-full` or `rustforge-full` still transitively compiles all
   78 extension crates.

---

## 6. Summary

| Item | Answer |
|---|---|
| Core count | 34 stable crates (27 clean library + 3 buggy library + 2 umbrellas + 2 CLIs) |
| Extension count | 78 (70 beta + 8 experimental) |
| Stub count | 9 (non-workspace, candidates for deletion) |
| Closed-set holds? | For 27/34 stable crates. 3 library crates violate it (bugs); 4 violate it by design (umbrellas, CLIs). Must fix 3 before extraction is meaningful. |
| Umbrella plan | `rf` → stable-core-only; `rf-full` (new, beta) → everything; same for `rustforge`/`rustforge-full` |
| Recommendation | Option C (hybrid): in-repo split now, separate repo after crates.io publication |
| Pilot this cycle | This plan document only. Phase 1 (experimental crate move) begins Cycle 20. |
| Reversibility | Phases 1–3 fully reversible in-repo. Phase 4 is the one-way door — deferred to maintainer decision post-publication. |
