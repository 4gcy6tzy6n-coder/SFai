# System for future ai (sfai) — Rust

A general-purpose, **model-native agent platform**: the main model chooses its
next action, capability, Skill, and information access — no mandatory
Planner → Executor → Reviewer workflow, no external LLM router. The runtime
provides stable meta-operations, a discoverable capability fabric,
**index-native context**, policy-aware execution, and independent verification.

> Development process: every completed development increment must pass a
> **mandatory debug phase** (reproduce → isolate → root-cause → fix →
> regression → record). See
> [`docs/development-debug-process.md`](docs/development-debug-process.md).

## Status

Early implementation of engineering plan v0.5 (`docs/implementation-plan.md`),
issues **SF-001 – SF-014** (milestones M0–M3 core), written in Rust
(superseding the plan's Python baseline — see ADR-012):

| Area | What works today |
|---|---|
| Contracts | Typed serde entities (Task/TaskState/Event/SourceRef/ActionIntent/ToolResult/CapabilityManifest), strict unknown-field rejection |
| Kernel | Self-routing decision loop with **direct-answer mode** (zero discovery calls) |
| Broker | Policy risk tiers, approval-gated external writes, idempotent execution, audited denials, receipts |
| Storage | SQLite (WAL) event ledger, checkpoint/resume, content-addressed blob store |
| Context | Root/sub-index, FTS5 search, bounded `context.read`, working-set compiler with retrieval receipt |
| Verify | Deterministic acceptance checks with machine-readable diagnostics |
| Verify (M6) | code tests in the task workspace, JSON/document checks, risk-based mandatory verification after consequential actions, bounded repair with visible partial failure |
| Wiki (M4) | Incremental derived compiler: material-change classification, typed edges, provenance, contradiction edges, stale-ref flags, write-amplification report |
| Skills (M5) | `SKILL.md` packages: metadata-only discovery, per-run activation into the working set, version pinning, trust metadata |
| Telemetry | JSONL traces, token/cost accounting, trace replayer |
| Interfaces | CLI (`sfai run/resume/status/inspect/context/capabilities/skills/wiki/verify/export-trace`) |

Not yet (tracked in the debug reports / plan): end-to-end acceptance against a
real hosted model (offline fakes + mock HTTP only so far), OS-level sandboxing
for code checks (subprocess + workspace only), multi-statement DB transactions,
per-tenant ACL filtering (M7), learned sparse indexer (M8), web UI.

## Quick start

```bash
cargo test                        # offline correctness suite, no API keys
cargo run -- run --fixture tool-use
cargo run -- run --fixture direct-answer
cargo run -- status <task_id>
cargo run -- export-trace <task_id>
cargo run -- wiki summary --task <task_id>       # M4 derived knowledge
cargo run -- skills list --skills skills         # M5 skill discovery (metadata only)
```

Every run is fully offline and deterministic. Real-model runs use any
OpenAI-compatible endpoint via `SFAI_MODEL_*` (see `.env.example`); the client
is exercised against a local mock server in the test suite.

## Layout

```
Cargo.toml                      # workspace
crates/sfai/
├── src/
│   ├── contracts.rs            # persistent entity contracts (plan §3.1)
│   ├── ids.rs                  # ids, refs, provenance ref extraction
│   ├── core/                   # decision kernel, model adapter, intents, envelope, termination
│   ├── authority/              # task store, approvals
│   ├── context/                # index, retrieval (FTS5), working set
│   ├── capabilities/           # registry: discover/inspect/activate
│   ├── runtime/                # policy engine, policy-aware broker
│   ├── verify/                 # deterministic verification + diagnostics
│   ├── integrations/           # fixture tools, read-only fs, provider adapters
│   ├── storage/                # SQLite db, event ledger, object store
│   ├── telemetry/              # tracing, cost, replay
│   ├── app.rs                  # composition root
│   ├── fixtures.rs             # offline task fixtures
│   └── bin/sfai.rs             # CLI
└── tests/                      # integration / security / replay / e2e suites
```

## Design rules (non-negotiable)

1. One primary decision model; no mandatory external LLM router.
2. Tools, capabilities, Skills and models are different objects.
3. Self-routing is genuine: answer / discover / activate / retrieve / execute / verify / ask / pause / finish.
4. No index-only amnesia: goal, constraints, permissions and status stay pinned.
5. The Wiki is a derived representation, never the source of truth.
6. Verification is domain-general and independent of routing.
7. All costs are measured, including decision overhead.
8. Retrieved text is data — it can never grant permissions.

See `docs/implementation-plan.md` for the full plan and `docs/decisions/` for
ADRs (language switch, Rust layout decisions).
