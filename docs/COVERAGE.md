# RustForge Stable-Core Coverage Report

**Generated**: 2026-07-14 (cycle 8)
**Tool**: `cargo-llvm-cov 0.8.7` with Xcode LLVM toolchain
**Caveat**: Numbers reflect tests that compile and run without external
dependencies (no live Redis, PostgreSQL, AWS). Database-driver paths
(rf-orm query_builder, transaction, sharding/manager), Redis paths, and
SQS paths show low coverage for structural reasons: they require
infrastructure not present in CI. This is an honest view, not a green-washed
summary.

---

## Reproduction command

```sh
export LLVM_PROFDATA=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-profdata
export LLVM_COV=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-cov
# Install cargo-llvm-cov once if not present:
#   cargo install cargo-llvm-cov
cargo llvm-cov --summary-only -p rf-queue
cargo llvm-cov --summary-only -p rf-validation
cargo llvm-cov --summary-only -p rf-auth
cargo llvm-cov --summary-only -p rf-web
cargo llvm-cov --summary-only -p rf-cache
cargo llvm-cov --summary-only -p rf-orm
```

---

## Per-crate summary (lines covered)

| Crate           | Lines Covered | Lines Total | Line % | Functions % | Notes |
|-----------------|--------------|-------------|--------|-------------|-------|
| **rf-queue**    | 555 / 722    | 722         | **76.9%** | 70.5% | +14 new integration tests added in cycle 8 |
| **rf-validation** | 1870 / 2376 | 2376       | **78.7%** | 71.6% | Rules well-tested; database rules partial (needs DB) |
| **rf-auth**     | 1962 / 2469  | 2469        | **79.5%** | 77.7% | JWT, password, CSRF solid; remember-me middleware thin |
| **rf-web**      | 2041 / 2299  | 2299        | **88.8%** | 86.2% | Strongest crate; timeout middleware is the main gap |
| **rf-cache**    | 928 / 1126   | 1126        | **82.4%** | 74.1% | Memory path well covered; Redis paths excluded |
| **rf-orm**      | 3470 / 7090  | 7090        | **48.9%** | 48.9% | Honest low: query_builder, transaction, relationships require a live DB |

---

## rf-queue file breakdown (cycle-8 after new tests)

| File              | Lines % | Regions % | Key gap |
|-------------------|---------|-----------|---------|
| `job.rs`          | 94.5%   | 91.9%     | Minimal; edge-case error branches |
| `facade.rs`       | 89.2%   | 88.4%     | Good; `dispatch_later` delay path minor gap |
| `memory.rs`       | 82.1%   | 87.9%     | `fail` fallback (direct-fail without reserve) |
| `drivers/failover.rs` | 80.9% | 85.2%  | Healthy; multi-backend fallover path |
| `api.rs`          | 74.6%   | 71.6%     | Sync bridge paths exercised; `reserve`/`complete`/`fail` via facade untested |
| `worker.rs`       | 60.3%   | 64.0%     | Timeout path, `start()`/`run_loop()` not exercised (need long-running test) |
| `config.rs`       | 68.3%   | 76.8%     | Redis/SQS config arms (feature-gated) |
| `queue.rs`        | 0.0%    | 0.0%      | Trait definition only; no code to cover |

### Known gaps and honest rationale

- **worker.rs 60%**: The `start()` + `run_loop()` paths require a long-running
  worker; they are intentionally not unit-tested to keep tests fast. The
  `work_once()` path (the critical drain API) is fully covered by cycle-8 tests.
- **worker.rs timeout path**: Covered in theory by the `tokio::time::timeout`
  wrap but no test currently injects a slow-enough job to trigger it without
  introducing `sleep` in tests.
- **rf-orm 49%**: The gap is structural: `query_builder.rs` (10.8% covered) and
  `transaction.rs` (13.2%) require a live PostgreSQL instance. These are not
  false-positive passes — they are genuinely uncovered.

---

## Bugs found during cycle-8 test writing

| Bug | Status |
|-----|--------|
| `OK_RUNS` global counter raced between `work_once_executes_job_body_and_completes` and `panic_in_job_is_isolated_worker_survives` tests when run in parallel — revealed that relying on process-global state in integration tests is fragile | Fixed: both tests now use queue-state assertions only |

No production logic bugs were found in rf-queue. All 14 new tests (plus the existing 19) pass green.
