# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T12:51:28.080391+00:00
- tasks per suite per repetition: 2
- repetitions: 1
- base seed: 7
- config: e2m-deepseek-v4-pro
- provider: bailian

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | no_verification | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 120269.27 | 120269.27 | 95221.02 ± 49094.57 | 8604 | 2945 | 0.000000 ± 0.000000 | 0.000000 |
| dataset | none | 2 | 0.50 [0.09,0.91] | 0.50 [0.09,0.91] | 0.00 | 148988.01 | 148988.01 | 143431.42 ± 10890.90 | 12564 | 4178 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 37212.19 | 37212.19 | 35547.96 ± 3261.89 | 3103 | 1096 | 0.000000 ± 0.000000 | 0.000000 |
| edge-case | none | 2 | 0.00 [0.00,0.66] | 0.00 [0.00,0.66] | 0.00 | 83026.35 | 83026.35 | 82172.80 ± 1672.95 | 5644 | 2686 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 49039.02 | 49039.02 | 45495.42 ± 6945.46 | 5751 | 1532 | 0.000000 ± 0.000000 | 0.000000 |
| long-horizon | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 149978.00 | 149978.00 | 135417.75 ± 28538.10 | 16650 | 4534 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | no_verification | 2 | 1.00 [0.34,1.00] | 0.00 [0.00,0.66] | 1.00 | 191852.17 | 191852.17 | 156134.63 ± 70006.38 | 13310 | 4904 | 0.000000 ± 0.000000 | 0.000000 |
| tool-use | none | 2 | 1.00 [0.34,1.00] | 1.00 [0.34,1.00] | 0.00 | 334463.45 | 334463.45 | 248331.53 ± 168818.58 | 17989 | 7845 | 0.000000 ± 0.000000 | 0.000000 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | no_verification | 7.5 | 7.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 11.0 | 10.5 | 0 | 3 | 0 | 0.00 |
| edge-case | no_verification | 3.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| edge-case | none | 5.0 | 2.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | no_verification | 3.5 | 2.5 | 0 | 1 | 0 | 0.00 |
| long-horizon | none | 9.0 | 7.0 | 0 | 2 | 0 | 0.00 |
| tool-use | no_verification | 10.0 | 9.0 | 1 | 2 | 0 | 0.00 |
| tool-use | none | 12.5 | 10.5 | 1 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e2m-deepseek-v4-pro.json
