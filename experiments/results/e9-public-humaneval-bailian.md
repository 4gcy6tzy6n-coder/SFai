# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-17T03:49:47.018585+00:00
- tasks per suite per repetition: 20
- repetitions: 1
- base seed: 0
- config: e9-public-humaneval-bailian
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| public-humaneval | no_verification | 20 | 0.80 [0.58,0.92] | 0.00 [0.00,0.16] | 0.80 | 106595.78 | 144898.14 | 90890.75 ± 20975.23 | 31005 | 13867 | 0.000000 ± 0.000000 | 0.000000 |
| public-humaneval | none | 20 | 0.00 [0.00,0.16] | 0.00 [0.00,0.16] | 0.00 | 159930.02 | 205121.63 | 159907.08 ± 15396.68 | 46175 | 23851 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| public-humaneval | no_verification | 15.9 | 15.7 | 20 | 4 | 10 | 0.00 |
| public-humaneval | none | 23.4 | 20.6 | 28 | 14 | 19 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e9-public-humaneval-bailian.json
