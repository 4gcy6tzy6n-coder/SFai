# Research Plan — dual deliverables (agent application + top-tier paper)

**Status:** living document (v0.1, 2026-11-16)
**Supersedes:** nothing; operationalizes plan §13 (`docs/implementation-plan.md`)
**Owner rule:** this project has **two deliverables** — (1) a usable agent
application, (2) a top-tier research paper backed by completed experiments.
Neither is optional, and the experiment track is a first-class workstream, not
an afterthought.

## 1. Two tracks, one codebase

| Track | Deliverable | Current state | Next gate |
|---|---|---|---|
| Product | Usable agent application (CLI + UI + `sfai serve` API) | M0–M7 backend done; CLI UI in progress; desktop UI planned; `.dmg` packaging planned | UI integration + packaging |
| Research | Top-tier paper + reproducible experiments | M8 baseline harness v0 (`sfai eval`); synthetic fixtures; no real-model runs yet | real-model pilot → ablations → paper draft |

**Engineering contributions** (product): capability fabric, model-native control
loop, durable Wiki/index store, provenance, production runtime.
**Scientific contribution** (paper, unproven until evaluated): a state-aware
*sparse context activation policy* — optionally jointly conditioned on
model-selected capabilities — that improves long-horizon task effectiveness at
lower end-to-end cost, against strong context/memory/Skill-selection baselines.

## 2. Research questions (plan §13.2)

- **RQ1 — Index-resident context.** Can an agent operate with bounded resident
  context while retaining essential task information and improving
  full-system cost–quality trade-offs?
- **RQ2 — Execution-aware retrieval.** Does conditioning retrieval on task
  state and current capability outperform ordinary semantic/lexical retrieval?
- **RQ3 — Self-routing + context.** Is joint capability/context selection
  better than independent routing + retrieval under matched models/tools?
- **RQ4 — Robustness.** Which failures come from omitted, stale, contradictory,
  unauthorized, or over-compressed context, and how are they detected?

Hypotheses are **unproven**; the harness measures them, it does not settle them.

## 3. Baselines and ablations (plan §13.3)

Baselines (all matched for model, tools, permissions, budgets, caching):

1. Full conversation/tool history.
2. Fixed sliding context window.
3. Summary/compaction baseline.
4. Conventional RAG over raw history.
5. Hierarchical Wiki index, on-demand retrieval.
6. Execution-state-aware sparse selection.
7. Capability-aware sparse selection.
8. Joint capability/context selection.
9. Routing comparisons: rule router, separate LLM router, native self-routing.
10. Ablations: no Wiki compilation, no graph edges, no pinned constraints, no
    hot cache, fixed top-k vs token budget, no verification.

## 4. Metrics (plan §13.4)

| Dimension | Measures |
|---|---|
| Task performance | success rate, acceptance pass rate, artifact correctness |
| Cost | gross input/output tokens, cache usage, total cost, cost per successful task |
| Latency | end-to-end seconds, first useful action, tool/compilation/retrieval time |
| Context quality | critical fact recall, constraint retention, stale-ref rate |
| Routing | misroutes, unnecessary activations, Skill load count, duplicate calls |
| Reliability | crash recovery, retry duplication, false-success rate, ACL violations |
| Scale | context size, indexed documents, retrieval latency, write amplification |

A lower prompt size alone is **not** proof of lower end-to-end cost; indexing,
verification, retries and wall-clock must be charged.

## 5. Experiment protocol

1. **v0 (current): deterministic offline harness.** Synthetic long-horizon
   fixtures, five context policies simulated at the context layer, a
   deterministic answer simulator, no network. Purpose: instrument the metrics
   and freeze the protocol before touching real models.
2. **v1: real-model pilot.** Same task schemas, hosted model via the provider
   adapter, repeated seeds (≥5 per condition), frozen model versions, report
   mean ± 95% CI.
3. **v2: ablations + Wiki/Skill integration.** Turn on/off Wiki compilation,
   graph edges, pinned constraints, verification; measure both quality and
   cost.
4. **Experiment registry.** Executable configs live in `experiments/configs/`
   (E1 DeepSeek baseline, E2 provider comparison, E4 ablations); protocol and
   controls in `docs/experiments/README.md`; runner: `scripts/run_experiments.sh`.
5. **v3: paper-scale.** Public reproducible tasks plus internal fixtures,
   independent acceptance checks (never the same model's self-assessment),
   negative results retained, full artifact release.

## 6. v0 harness scope and honest limitations

`sfai eval` (crate module `eval`) runs `sparse-context` suites of synthetic
scenarios and reports, per policy: success rate, needle recall, constraint
retention, mean input tokens, mean cost, cost per success, mean latency.

Limitations that must be stated in any write-up:

- the answer is produced by a **deterministic simulator**, not a model;
- the tasks are **synthetic fixtures**, not public benchmarks;
- policies are simulated at the **context-selection layer**, not end-to-end;
- results establish that the measurement pipeline works and that pinned
  constraints behave as designed — **not** evidence of a paper contribution.

## 6.5 Positioning (revised after gap analysis + E6 overhead measurements)

**Primary contribution: a complete, auditable agent runtime + benchmark
("Agent Artificial Analysis"-style), engineered in Rust.** The unit of
comparison is the *whole pipeline* (self-routing kernel, index-native context,
runtime-enforced verification, multi-provider, CLI/UI/dmg, benchmark with
false-success metric), not a single algorithm. Speed claims are scoped
honestly (E6): parity per-task overhead at equal guarantees, 2–3× cold start,
5× large-envelope serialization, 10×+ distribution size, concurrency TBD (E6b).
Secondary: the verification-gate / false-success quantification across models
(E1/E2/E4) and sparse-context cost evidence (E4) as empirical findings.

## 7. Paper gate (do not freeze a claim until all hold)

- the method is described mathematically and differs materially from known
  Wiki/compaction/memory/Skill-routing methods;
- at least one strong alternative is reproduced fairly;
- the policy beats or meaningfully extends a quality–cost Pareto frontier, or
  explains a robust, novel trade-off;
- reproducible code, traces, ablations and failure analysis exist.

Venue acceptance is not guaranteed; negative results remain in reports.

## 8. Definition of done (both tracks)

**Product:** usable agent app (CLI + UI + API), packaged and usable.
**Research:** novel mechanism specified, controlled baselines + cross-domain
tasks support the claim, uncertainty/failure modes/costs reported, code +
protocol + traces reproducible.
