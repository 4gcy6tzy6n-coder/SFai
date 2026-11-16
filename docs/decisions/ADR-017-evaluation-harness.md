# ADR-017: Offline sparse-context evaluation harness (M8 v0)

**Date:** 2026-11-16
**Status:** Accepted for v0 — protocol frozen, results not paper evidence
**Context:** The project has two deliverables (agent application + top-tier
paper). Experiments must be reproducible and comparable before real-model runs
are meaningful. This ADR records the v0 harness.

## Decisions

1. **Synthetic, deterministic fixtures.** `generate_suite(size)` builds 40-doc
   histories with a needle at early/middle/late positions and a separate
   "Task constraints:" document. No network, no randomness: identical inputs
   produce identical metrics (wall-clock latency excluded from equality tests).
2. **Policies simulated at the context-selection layer.**
   `full_history`, `sliding_window(6)`, `summary` (first sentence of
   query-matching docs, top 3), `retrieval_topk(4)`, and `working_set` (the
   production `compile_working_set` with a 120-token budget). This isolates
   context policy from model variance for the first pass.
3. **Deterministic answer simulation.** Success requires the needle document
   *and* constraint retention; the simulated answer consumes a fixed 20 output
   tokens. The harness is honest about this (report notes + research plan §6).
4. **Metrics per policy.** success rate, needle recall, constraint retention,
   mean input tokens, mean cost, cost per success, mean latency; aggregated
   over scenarios, plus per-scenario outcomes in the JSON report.
5. **Reports.** `sfai eval` prints Markdown and writes JSON
   (`<storage>/evals/report-<ts>.json` by default, `--out` overrides). The v0.1
   baseline is archived at `evals/reports/v0.1-sparse-context-baseline.md` +
   `.json`.
6. **Constraints are pinned only by `working_set`.** This demonstrates the
   designed invariant: pinned constraints are never evicted; unpinned policies
   lose them when they drop early context. It is a property of the fixture
   design, not a discovered empirical fact.

## v0.1 baseline (12 scenarios, 120-token budget)

| policy | success | needle recall | constraint retention | mean input tokens |
|---|---|---|---|---|
| full_history | 1.00 | 1.00 | 1.00 | 810.0 |
| working_set | 1.00 | 1.00 | 1.00 | 107.0 |
| sliding_window | 0.00 | 0.33 | 0.00 | 121.3 |
| summary | 0.00 | 1.00 | 0.00 | 24.0 |
| retrieval_topk | 0.00 | 1.00 | 0.00 | 24.0 |

Interpretation: the harness shows the intended measurement pipeline works and
that working-set selection matches full-history success at ~13% of the input
tokens. It does **not** establish a paper contribution: the answer is simulated
and the tasks are synthetic.

## Limitations / next steps

- Real-model runs (v1) with repeated seeds and confidence intervals.
- Public reproducible tasks and cross-domain coverage (v2/v3).
- Routing and capability-conditioned baselines (plan §13.3 items 6–9).
- Ablations (Wiki/graph/pinned constraints/verification/hot cache).
- Budget sweeps (fixed top-k vs token budget) and Pareto curves.

## Consequences

- The protocol is frozen for v1 so real-model results are comparable.
- Any change to scenario generation or metric definitions must bump the
  harness version and be recorded here.
