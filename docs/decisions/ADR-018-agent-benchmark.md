# ADR-018: Agent framework benchmark (v0.1)

**Date:** 2026-11-16
**Status:** Accepted for v0 — protocol frozen for v1 real-model runs
**Context:** The project's second deliverable (a top-tier paper) requires
comparable evidence that the framework is efficient and correct under cost
constraints. The benchmark must measure **time, task completion, correctness,
tokens, cost and ablations end-to-end**, not just context-layer policy
simulation (which the `sfai eval` harness already covers).

## Decisions

1. **End-to-end measurement through the real kernel.** Each task runs against a
   fresh temporary storage root and exercises stores, broker, verification,
   Wiki compilation and the cost ledger. v0 uses deterministic scripted models
   so the framework can be measured without model variance; v1 swaps in hosted
   models.
2. **Four suites, four ground-truth shapes.** `tool-use` (cited report),
   `dataset` (computed total in JSON), `long-horizon` (needle recovered from 40
   notes), `edge-case` (deliberately broken citation). Correctness is checked
   against an artifact-scoped ground-truth marker so seeded inputs cannot
   accidentally satisfy it.
3. **Five ablations as first-class flags.** `none`, `full_context`,
   `no_pinned`, `no_verification`, `no_wiki` map to existing switches
   (`candidate_limit`, `pin_constraints`, `enforce_acceptance`,
   `wiki_compile_enabled`). This keeps one kernel and one code path.
4. **False-success metric.** `completed && !(acceptance && correct)` is reported
   separately: it measures the safety cost of a missing verification gate, not
   just task failure. The `edge-case × no_verification` cell is the reference
   demonstration (100% false success in v0.1).
5. **Latency p50/p95 and per-success cost/token metrics**, not just means, so
   tails and efficiency are visible.
6. **Deterministic equality tests exclude wall-clock latency**; all other
   metrics must be identical across runs with the same inputs.
7. **Reports:** Markdown to stdout, JSON to `<storage>/benchmarks/...` (or
   `--out`); the v0.1 baseline is archived in `evals/reports/`.

## v0.1 baseline highlights

- `long-horizon`: full-context ablation uses ~2.2× the input tokens of the
  governed envelope (4843 vs 2243) at the same success/correctness.
- `edge-case`: with verification, completion is blocked (0% false success);
  with `no_verification`, the broken deliverable is reported as success
  (100% false success) while costing *less* latency — the classic trade-off the
  benchmark is meant to expose.
- `dataset`: exact ground-truth totals pass under all ablations; the suite
  exercises artifact publishing and JSON validation.

## Limitations / next steps

- v0 models are scripted: results measure framework overhead, not model
  quality; do not compare them to model benchmarks.
- Suites are synthetic; v1 adds real models and public tasks, v2 adds
  Wiki/Skill-conditioned variants, v3 adds Pareto sweeps and confidence
  intervals.
- Latencies are machine-specific; reports must include hardware and treat
  absolute numbers as relative indicators.

## Consequences

- The protocol is frozen; any suite or metric change bumps `BENCH_VERSION` and
  is recorded here.
- The research plan (§5) maps v1/v2/v3 onto this harness.
