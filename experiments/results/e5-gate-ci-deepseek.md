# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T11:59:05.883336+00:00
- tasks per suite per repetition: 4
- repetitions: 4
- base seed: 42
- config: e5-gate-ci-deepseek
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| edge-case | no_verification | 16 | 0.38 [0.18,0.61] | 0.00 [0.00,0.19] | 0.38 | 5390.02 | 6164.06 | 4737.28 ± 680.35 | 3160 | 627 | 0.001543 ± 0.000207 | 0.004115 |
| edge-case | none | 16 | 0.00 [0.00,0.19] | 0.00 [0.00,0.19] | 0.00 | 5107.25 | 13712.96 | 6282.10 ± 1748.83 | 4461 | 888 | 0.002181 ± 0.000626 | 0.000000 |
| long-horizon | no_verification | 16 | 1.00 [0.81,1.00] | 0.62 [0.39,0.82] | 0.38 | 9583.04 | 20496.01 | 11804.08 ± 2894.19 | 13019 | 1750 | 0.005441 ± 0.001309 | 0.005441 |
| long-horizon | none | 16 | 0.94 [0.72,0.99] | 1.00 [0.81,1.00] | 0.00 | 14679.94 | 27312.05 | 15578.26 ± 3263.14 | 18351 | 2481 | 0.007684 ± 0.001561 | 0.008196 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| edge-case | no_verification | 3.1 | 2.1 | 0 | 0 | 0 | 0.62 |
| edge-case | none | 4.1 | 2.8 | 0 | 0 | 0 | 0.94 |
| long-horizon | no_verification | 7.5 | 6.5 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 8.8 | 8.8 | 1 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e5-gate-ci-deepseek.json
