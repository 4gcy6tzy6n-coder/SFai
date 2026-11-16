# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:04:10.820457+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-qwen3-8-max
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 139690.94 | 139690.94 | 118145.88 ± 42228.31 | 9524 | 4198 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 113596.61 | 113596.61 | 86871.94 ± 52380.36 | 7758 | 3154 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.50 [0.09,0.91] | 0.00 [0.00,0.66] | 0.50 | 75359.11 | 75359.11 | 62232.25 ± 25728.64 | 4809 | 2251 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 564330.08 | 564330.08 | 308778.90 ± 500880.32 | 16518 | 12052 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 433102.00 | 433102.00 | 257789.53 ± 343612.45 | 35608 | 10280 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 83511.10 | 83511.10 | 79546.72 ± 7770.19 | 11792 | 3072 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 177350.45 | 177350.45 | 163196.09 ± 27742.55 | 16170 | 6101 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 628838.53 | 628838.53 | 491057.31 ± 270051.18 | 33274 | 18926 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 8.0 | 7.0 | 2 | 0 | 0 | 0.00 |
| dataset | none | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 4.5 | 4.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 12.5 | 11.0 | 0 | 0 | 0 | 0.50 |
| long-horizon | no_verification | 17.5 | 16.5 | 1 | 9 | 0 | 0.00 |
| long-horizon | none | 6.5 | 5.5 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 11.5 | 10.5 | 0 | 2 | 0 | 0.00 |
| tool-use | none | 21.5 | 20.0 | 1 | 5 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-qwen3-8-max.json
