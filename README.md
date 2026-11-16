# System for future ai (sfai) — Rust

A general-purpose, **model-native agent platform**: the main model chooses its
next action, capability, Skill, and information access — no mandatory
Planner → Executor → Reviewer workflow, no external LLM router.

**Dual deliverables (non-negotiable):**
1. a usable agent **application** (CLI + UI + local API), and
2. a **top-tier research paper** supported by completed experiments
   (see [`docs/research-plan.md`](docs/research-plan.md)). The runtime
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
| API (M7) | local HTTP + SSE service (`sfai serve`): tasks, events, run/cancel, artifacts, trace, wiki, verify |
| Research (M8) | offline sparse-context harness (`sfai eval`): 5 context policies, success/recall/constraint/token/cost metrics, JSON+Markdown reports |
| Benchmark (M8+) | agent framework benchmark (`sfai bench`): tool-use / dataset / long-horizon / edge-case suites × 5 ablations; success, correctness, false-success, latency p50/p95, tokens, cost |

Not yet (tracked in the debug reports / plan): end-to-end acceptance against a
real hosted model (offline fakes + mock HTTP only so far), OS-level sandboxing
for code checks (subprocess + workspace only), multi-statement DB transactions,
per-tenant ACL filtering (M7), learned sparse indexer (M8), web UI.

## Quick start

```bash
cargo test                        # offline correctness suite, no API keys
cargo run                         # bare sfai: interactive session
cargo run -- run --fixture tool-use
cargo run -- run --fixture direct-answer
cargo run -- status <task_id>
cargo run -- export-trace <task_id>
cargo run -- wiki summary <task_id>             # M4 derived knowledge
cargo run -- skills list --skills skills         # M5 skill discovery (metadata only)
cargo run -- eval --size 12                      # M8 offline evaluation harness
cargo run -- bench --size 4                     # agent framework benchmark (all suites/ablations)
cargo run -- bench --suite dataset --ablation none --out /tmp/bench.json
```

## HTTP API (for the UI)

```bash
cargo run -- serve --port 8765          # binds 127.0.0.1 by default
SFAI_API_TOKEN=secret cargo run -- serve # optional bearer-token guard
```

Endpoints (JSON unless noted; when `SFAI_API_TOKEN` is set, every `/api/*`
route except `/api/health` requires `Authorization: Bearer <token>`):

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/health` | health + fixture list |
| GET | `/api/capabilities` | capability manifests |
| GET | `/api/skills` | skill metadata only (no instructions) |
| GET/POST | `/api/tasks` | list / create `{goal, constraints?, acceptance_rules?, max_tool_calls?}` |
| GET | `/api/tasks/:id` | task + state |
| GET | `/api/tasks/:id/events?after=<seq>` | ledger events |
| GET | `/api/tasks/:id/stream` | SSE stream (`ledger` events + heartbeats) |
| POST | `/api/tasks/:id/run` | run `{fixture?, max_steps?}` (fixture model or provider) |
| POST | `/api/tasks/:id/cancel` | cooperative cancellation |
| GET | `/api/tasks/:id/artifacts` | artifact metadata |
| GET | `/api/tasks/:id/artifacts/:artifact_id` | artifact text |
| GET | `/api/tasks/:id/trace` | trace entries |
| GET | `/api/tasks/:id/wiki` | derived Wiki summary |
| GET | `/api/tasks/:id/verify` | acceptance verification results |
| POST | `/api/demo/:fixture` | create + run a demo task |

Duplicate runs of the same task are rejected with `409`; the kernel checks the
cancel flag each step, so cancelled tasks end in `cancelled` rather than
silently finishing.

## Real model providers (DeepSeek / Qwen / MiMo)

```bash
export DEEPSEEK_API_KEY=sk-...
cargo run -- run --goal "research X and write a cited report" --provider deepseek --model deepseek-chat
cargo run -- serve --provider deepseek          # API defaults to DeepSeek
cargo run -- bench --provider deepseek --model deepseek-chat \
  --suite dataset --ablation none --size 3 --out evals/reports/v1-deepseek.json
```

Profiles, env vars, pricing overrides and troubleshooting:
[`docs/providers.md`](docs/providers.md). Custom/self-hosted endpoints use
`SFAI_MODEL_BASE_URL` + `SFAI_MODEL_NAME` + `SFAI_MODEL_API_KEY`.

## Agent framework benchmark

`sfai bench` measures the framework end-to-end (kernel, tools, stores,
verification, Wiki) on four suites and five ablations, reporting success,
ground-truth correctness, **false-success**, p50/p95 latency, input/output
tokens, and cost. Protocol and v0.1 baseline:
[`docs/benchmark-spec.md`](docs/benchmark-spec.md). Archived reports:
`evals/reports/`.

### Interactive session

Bare `sfai` (no subcommand) opens a small REPL:

```text
sfai> /run tool-use          # run an offline fixture
sfai> <type any goal>        # run it with the configured provider
sfai> /status /trace <id> /wiki <id> /skills /fixtures /help /quit
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
