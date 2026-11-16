# Experiments — registry and protocol

**Status:** v0.1 (2026-11-16). Experiment configs are executable; runs require a
provider API key. Results are archived under `experiments/results/`.

## Registry

| ID | Question | Config | Status |
|---|---|---|---|
| E1 | Does the framework complete real-model tasks correctly, and at what time/token/cost? | `deepseek-e1.json` | pending key |
| E2 | How do providers compare on the same suites (DeepSeek / Qwen / MiMo)? | `qwen-e2.json`, `mimo-e2.json` | pending keys |
| E3 | Does sparse context activation beat full history on quality-cost? | `sfai eval` + E4 | harness ready (v0), real-model pending |
| E4 | What do framework components contribute (Wiki, pinned constraints, verification, full context)? | `deepseek-ablations.json` | pending key |
| E5 | Scale/Pareto: budget vs quality vs latency; fixed top-k vs token budget | planned | planned |

## Controls (must hold for any comparison)

- Same provider/model version, same task config, same seed set, same budgets.
- Same price table (`price_in`/`price_out` recorded in the config).
- Same hardware; report machine, OS, Rust version, and date.
- Repetitions ≥ 2 for real models; report mean and 95% CI (benchmark emits
  Wilson intervals for rates and normal-approx intervals for latency/cost).
- Do not mix v0 (scripted) and v1 (real) numbers in one table without labels.

## Protocol

```bash
# 1. export the provider key (never commit it)
export DEEPSEEK_API_KEY=sk-...

# 2. run one config (Markdown + JSON under experiments/results/)
cargo run --release -- bench --config experiments/configs/deepseek-e1.json \
  --storage experiments/.run/storage --out experiments/results/e1-deepseek-baseline.json

# 3. all configs with available keys
./scripts/run_experiments.sh
```

The script skips configs whose key is missing and never prints key values.

## Metrics (per suite × ablation)

- success rate (completed) with 95% Wilson CI
- ground-truth correctness (artifact-scoped) with 95% CI
- false-success rate (completed but acceptance failed or deliverable wrong)
- latency p50/p95, mean ± 95% CI
- input/output tokens, mean cost ± 95% CI, cost per success, tokens per success

## Artifact policy

- Raw JSON per run: `experiments/results/<name>.json` (all task rows).
- Human-readable Markdown: `experiments/results/<name>.md`.
- Never edit result files by hand; regenerate from the config.
- Record the git commit (`git rev-parse HEAD`) next to each result when archiving.
