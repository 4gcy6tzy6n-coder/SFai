# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T13:03:55.912847+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-glm-5-2
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 36813.30 | 36813.30 | 34941.85 ± 3668.03 | 6651 | 2670 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 32886.83 | 32886.83 | 27724.23 ± 10118.70 | 5143 | 2080 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 0.50 [0.09,0.91] | 0.00 [0.00,0.66] | 0.50 | 23678.16 | 23678.16 | 22277.84 ± 2744.63 | 2550 | 1493 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 38144.48 | 38144.48 | 37935.57 ± 409.45 | 4121 | 2584 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 55835.87 | 55835.87 | 50532.92 ± 10393.77 | 9782 | 4412 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 60351.46 | 60351.46 | 52458.53 ± 15470.15 | 12894 | 4058 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.50 [0.09,0.91] | 0.50 | 72539.38 | 72539.38 | 54946.65 ± 34481.75 | 10610 | 4301 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 110405.61 | 110405.61 | 93005.03 ± 34105.15 | 14366 | 7425 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 7.0 | 6.0 | 2 | 0 | 0 | 0.00 |
| dataset | none | 5.5 | 5.0 | 0 | 0 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.50 |
| edge-case | none | 4.5 | 2.0 | 0 | 0 | 0 | 1.00 |
| long-horizon | no_verification | 6.0 | 5.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 8.0 | 6.5 | 0 | 0 | 0 | 0.00 |
| tool-use | no_verification | 9.0 | 8.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 11.5 | 9.5 | 0 | 3 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-glm-5-2.json
