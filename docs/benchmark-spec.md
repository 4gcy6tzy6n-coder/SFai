# Agent Framework Benchmark — v0.1 specification

**Status:** v0.1 (2026-11-16). Protocol is frozen for v1 real-model runs.
**Purpose:** a reproducible "Agent Artificial Analysis"-style benchmark for
agent frameworks: **time**, **task completion**, **ground-truth correctness**,
**token spend**, **cost**, **ablations** — measured end-to-end through the real
kernel (stores, policy broker, verification, Wiki), not a context-layer toy.

## 1. Tasks (suites)

| Suite | Goal | Acceptance rules | Ground truth |
|---|---|---|---|
| `tool-use` | research a topic and write a cited report | `artifact_exists:report.md`, `citations_resolve`, `source_count_at_least:3` | report contains "Citations" |
| `dataset` | compute total revenue from `sales.csv`, write `answer.json` | `artifact_exists:answer.json`, `json_valid:answer.json`, `artifact_contains:answer.json:<total>` | exact computed total |
| `long-horizon` | recover an earlier retention finding from 40 notes and quote it | `artifact_exists:report.md`, `artifact_contains:report.md:<marker>` | unique needle marker |
| `edge-case` | write a cited report | `artifact_exists:report.md`, `citations_resolve` | broken citation `source://missing@1` |

`dataset` totals and CSV rows are deterministic per task index. `long-horizon`
seeds 40 notes with the needle at position 20 (fits the working-set budget, so a
correct framework must retrieve it).

## 2. Metrics (per suite × ablation)

- **success_rate** — task status `completed` (framework claims success).
- **correctness** — expected artifact contains the ground-truth marker
  (artifact-scoped, so seeded data cannot accidentally satisfy it).
- **false_success_rate** — completed **but** acceptance failed or the deliverable
  was wrong. This is the key safety metric of the verification gate.
- **latency p50 / p95 / mean** — wall clock around `App::run` (kernel, tools,
  stores, verification, Wiki compile).
- **mean input/output tokens** — from the kernel cost ledger.
- **mean cost / cost per success / tokens per success** — priced via the cost
  table; per-success figures use the success rate as denominator.

## 3. Ablations

| Ablation | Change | What it demonstrates |
|---|---|---|
| `none` | full framework | reference |
| `full_context` | 100k-token envelope budget + 1000 retrieval candidates | cost of not compressing context |
| `no_pinned` | constraints not pinned into the envelope | risk when constraints can be evicted |
| `no_verification` | acceptance + mandatory-verification gates skipped | false-success risk |
| `no_wiki` | skip Wiki compilation at run end | latency/cost overhead of the derived store |

## 4. Protocol and reproducibility

- Every task runs in a fresh temporary storage root (isolated SQLite/blobs/workspaces).
- Deterministic scripted models; identical inputs produce identical metrics
  (wall-clock latency is the only non-deterministic field and is excluded from
  equality tests).
- Fixed budgets (default 20 tool calls), fixed model per suite, fixed seeds.
- CLI: `sfai bench [--suite ...] [--ablation ...] [--size N] [--out PATH]`
  prints Markdown and writes JSON.
- v0 measures **framework behaviour**, not model quality: the "model" is a
  scripted producer. v1 repeats the same suites with a hosted model through the
  provider adapter (≥5 seeds, confidence intervals).

## 5. v0.1 baseline (4 tasks per suite; archived in `evals/reports/`)

| suite | ablation | success | correct | false-success | p50 ms | p95 ms | in tokens | out tokens | cost USD |
|---|---|---|---|---|---|---|---|---|---|
| tool-use | none | 1.00 | 1.00 | 0.00 | 4.2 | 4.3 | 1062 | 254 | 0.001824 |
| tool-use | no_wiki | 1.00 | 1.00 | 0.00 | 3.1 | 3.2 | 1062 | 254 | 0.001824 |
| dataset | none | 1.00 | 1.00 | 0.00 | 4.0 | 4.0 | 494 | 88 | 0.000758 |
| long-horizon | none | 1.00 | 1.00 | 0.00 | 9.7 | 10.1 | 2243 | 200 | 0.002843 |
| long-horizon | full_context | 1.00 | 1.00 | 0.00 | 12.4 | 12.9 | 4843 | 200 | 0.005443 |
| edge-case | none | 0.00 | 1.00 | 0.00 | 5.9 | 6.2 | 1175 | 164 | 0.001667 |
| edge-case | no_verification | 1.00 | 1.00 | 1.00 | 3.2 | 3.6 | 465 | 96 | 0.000753 |

Reading: full-context costs ~2.2× the input tokens of the governed envelope on
long-horizon; removing the verification gate turns the deliberately broken
deliverable into a 100% false success (3.2 ms vs 5.9 ms p50 — verification has a
measurable but small latency cost at this scale).

## 6. Limitations (must be stated in any write-up)

- Scripted models: v0 isolates **framework overhead**, not model capability.
- Synthetic suites: not public benchmarks; `edge-case`/`no_verification` is a
  deliberately constructed failure demonstration, not an empirical finding.
- Latency is single-process, local SQLite, offline; absolute numbers are not
  comparable across machines — report relative comparisons and hardware.
- Cost is priced by the global price table; real provider invoices vary.
- v1 must add: real models, repeated seeds, confidence intervals, per-task
  traces, and hardware disclosure.
