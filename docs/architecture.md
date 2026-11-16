# Architecture (v0.1, Rust baseline)

This document maps the plan's architecture (§1 topology, §3 contracts) onto the
concrete Rust implementation. Deviations are recorded in `docs/decisions/`.

## Six logical planes — one process

| Plane | Rust modules | Responsibility |
|---|---|---|
| Model control | `core/decision_kernel.rs`, `core/model_adapter.rs`, `core/action_intents.rs` | choose next intent; normalize provider calls; stop when done |
| Capability & Skill | `capabilities/registry.rs`, `integrations/*` | discover/inspect/activate; handlers are pluggable closures |
| Context intelligence | `context/index.rs`, `context/retrieval.rs`, `context/working_set.rs`, `core/context_envelope.rs` | root index, FTS5 search, bounded reads, budgeted working set |
| Execution runtime | `runtime/broker.rs`, `runtime/policy.rs`, `integrations/fake_tools.rs` | policy-checked execution, approvals, receipts, idempotency |
| Verify & Debug | `verify/engine.rs`, `verify/diagnostics.rs` | deterministic acceptance checks; §7 diagnostic contract |
| Derived knowledge (M4) | `context/wiki.rs` | incremental Wiki: material-change classification, typed edges, provenance, contradictions, stale refs |
| Skill packages (M5) | `capabilities/skills.rs` | `SKILL.md` discovery (metadata only) and per-run activation into the working set |
| Infrastructure | `storage/db.rs`, `storage/event_ledger.rs`, `storage/objects.rs`, `telemetry/*` | SQLite WAL ledger, blob store, JSONL traces, cost accounting |

## Data flow

```text
user → App::create_task → TaskStore.create (event: task.created)
loop:
  TaskStore.state  ← fold(events)                      (source of truth)
  EnvelopeBuilder.build(state)                         (bounded envelope)
  ModelAdapter.decide(envelope) → ActionIntent         (one intent per step)
  │ respond/finish → AcceptanceEvaluator → Completed | open issue → retry
  │ ask_user / pause → visible lifecycle state
  └ execute → ToolBroker.execute                        (policy + approval + idempotency)
       → handler (capability closure) → ToolRun + event (tool.committed)
       → TaskStore.apply(state_events)                  (outputs, errors, issues)
  discover/retrieve/activate/verify → kernel meta dispatch + ledger event
  budgets → blocked when exceeded
```

## Key invariants and where they are enforced

| Invariant | Enforcement |
|---|---|
| Model cannot target another task | `DecisionKernel::run_task` rebinds `intent.task_id` before dispatch |
| Permission is never inferred from content | `PolicyEngine` reads only manifest `risk_class`; `ToolBroker` gates external writes on `ApprovalProvider` |
| Duplicate intents never commit twice | `tool_runs.idempotency_key UNIQUE` + replay of stored `ToolRun` |
| Denials are auditable | denials are committed as `ToolRun{status: denied}` + `tool.committed` events |
| No silent completion | acceptance rules must pass before `Completed`; failures become open issues and block after bounded retries |
| Events replay to the same state | `TaskStore.state` is a pure fold of the ledger; `resume()` compares replay against the task row |
| Costs are measured | `telemetry/cost.rs` counts model tokens, tool calls, verification runs, wall clock; trace replayer recomputes totals |
| Wiki never overrides authority | `context/wiki.rs` writes only wiki tables; compile failures are logged and the cursor is not advanced |
| Skill activation is per-run | `DecisionKernel::activated_skills`; envelopes include only activated skills, bounded to 4000 chars |
| Discovery leaks no instructions | `SkillManifest` has no instruction field; `SkillRegistry::discover` never reads the body |

## Storage strata (v0.1)

1. `tasks` table — canonical mutable task (goal/constraints/acceptance/status).
2. `events` — append-only ledger (source of truth) + `blobs/` content-addressed files.
3. `search_docs` + `search_fts` — derived lexical index (rebuildable).
4. `tool_runs`, `decisions`, `checkpoints` — receipts, decision audit, snapshots.
5. `wiki_nodes` / `wiki_edges` / `wiki_state` / `wiki_compile_log` — derived
   Wiki (M4) with provenance and write-amplification reports. Never a source of
   truth; a failed compile leaves the ledger and task authority untouched.

The Wiki compiler (M4) will add a derived layer above stratum 3; it must never
become a source of truth.

## Deviations from the plan's engineering defaults

- Language: **Rust** instead of Python 3.11 (user directive; ADR-012), keeping
  the same JSON contracts and semantics.
- Persistence: `rusqlite` (bundled SQLite, WAL) instead of SQLAlchemy — same
  contract-level behavior; see ADR-013.
- Kernel is synchronous in v0; `ModelAdapter` is a trait so an async executor
  can be added without changing contracts.
- Provider HTTP is implemented with `ureq` and tested against a local mock
  server; end-to-end runs against a hosted model remain unverified offline.
- Wiki compilation runs at task completion and on demand (`sfai wiki compile`);
  asynchronous/material-triggered scheduling is deferred (ADR-014).
