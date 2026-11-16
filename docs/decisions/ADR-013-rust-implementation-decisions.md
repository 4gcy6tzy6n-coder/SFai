# ADR-013: Rust implementation decisions (v0.1)

**Date:** 2026-11-16
**Status:** Accepted for v0.1 — revisit at M4/M7
**Context:** ADR-012 switched the implementation language to Rust. This ADR
records the concrete engineering choices taken while delivering SF-001–SF-014
(M0–M3 core) so later milestones do not re-litigate them silently.

## Decisions

1. **One crate, module layout mirrors the plan.**
   `crates/sfai/{contracts,ids,core,authority,context,capabilities,runtime,verify,storage,telemetry,integrations,app,fixtures,bin}`
   The workspace has a single member today; additional crates (e.g. a server
   binary, benchmarks) can be added without reshaping the contracts.
2. **Synchronous kernel; trait-based model adapter.**
   `ModelAdapter::decide(&ContextEnvelope) -> ModelResponse` is the single
   provider seam. The kernel loop is blocking and single-threaded in v0.
   Rationale: correctness-first, deterministic replay, no tokio dependency in
   the core. An async facade can wrap the same trait later.
3. **`rusqlite` (bundled SQLite) with a thin `Database` handle.**
   `Database` clones cheaply over `Rc<RefCell<Connection>>`; all SQL lives in
   storage modules. WAL is enabled; FTS5 is compiled in via `bundled`.
   Rationale: reproducible builds (no system SQLite dependency), same contract
   behavior as the plan's Python baseline (ADR-012 compatibility note).
4. **Single-statement writes; no multi-statement transactions yet.**
   Each event append is one atomic INSERT. Task creation writes the task row
   and its `task.created` event sequentially; a crash between them leaves a
   harmless task row that still replays as `created`. Cross-statement
   transactional groups are tracked as a follow-up issue (see debug report).
   Rationale: keeps the store adapters simple; `resume()` already detects
   materialization/replay divergence and refuses to paper over corruption.
5. **Model-visible envelope includes bounded recent tool results.**
   The plan requires short previews + refs (§4). `ContextEnvelope.recent_results`
   carries the last 5 `tool.committed` previews (≤2000 chars each). This is what
   makes rule-based demo models genuinely reactive instead of scripted.
6. **FTS5 queries are term-quoted.** Barewords containing `-`/`:` are parsed as
   operators by FTS5 (e.g. `sparse-context` → "no such column"), so the envelope
   builder wraps each OR-ed term in double quotes and falls back to a LIKE scan
   on any MATCH error.
7. **Provider HTTP deferred.** `openai_compat::normalize_response` is
   implemented and unit-tested offline; the live client returns an explicit
   `ModelError::Provider` until M1's real-model work. Core correctness must not
   depend on network access (plan §2 tests row).
8. **Multi-tenant ACL filtering is not implemented.** `context.search` can
   scope by task; tenant/user filtering before ranking is a M7 item (plan §5.5
   safety/ACL row). Single-process, single-user usage only in v0.

## Consequences

- Follow-up issues: add `rusqlite` transactions for task create/apply;
  implement provider HTTP; add ACL filters; add async facade; Wiki compiler.
- The mandatory debug phase (docs/development-debug-process.md) applies to every
  subsequent increment; the v0.1 debug report is in `docs/debug-reports/`.
