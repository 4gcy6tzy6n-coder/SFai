# Agent framework benchmark — v0.2.1

- generated_at: 2026-11-16T15:29:35.519357+00:00
- tasks per suite per repetition: 2
- repetitions: 2
- base seed: 11
- config: e8-routing-ablation-deepseek
- provider: deepseek

| suite | ablation | n | success (95% CI) | correct (95% CI) | false-success | p50 ms | p95 ms | mean latency ± 95% CI | in tokens | out tokens | cost USD ± 95% CI | cost/success USD |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| dataset | forced_pipeline | 4 | 0.75 [0.30,0.95] | 0.75 [0.30,0.95] | 0.00 | 5663.50 | 6885.91 | 5580.34 ± 1013.66 | 5184 | 821 | 0.002303 ± 0.000544 | 0.003071 |
| dataset | none | 4 | 0.25 [0.05,0.70] | 0.25 [0.05,0.70] | 0.00 | 9583.19 | 12362.57 | 9058.69 ± 3038.75 | 8338 | 1216 | 0.003589 ± 0.001298 | 0.014357 |
| long-horizon | forced_pipeline | 4 | 0.00 [0.00,0.49] | 0.00 [0.00,0.49] | 0.00 | 8965.70 | 10147.48 | 8963.67 ± 916.05 | 12154 | 1307 | 0.004719 ± 0.000434 | 0.000000 |
| long-horizon | none | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 11969.21 | 39767.37 | 18202.13 ± 14112.48 | 21876 | 2966 | 0.009170 ± 0.007461 | 0.009170 |
| tool-use | forced_pipeline | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 5216.04 | 5729.00 | 5231.54 ± 341.79 | 4956 | 854 | 0.002277 ± 0.000040 | 0.002277 |
| tool-use | none | 4 | 1.00 [0.51,1.00] | 1.00 [0.51,1.00] | 0.00 | 23797.31 | 30024.26 | 22219.56 ± 6287.67 | 19023 | 3548 | 0.009038 ± 0.002631 | 0.009038 |

## Routing quality

| suite | ablation | steps | tool calls | misroutes | loop-guard | dup replays | ask_user rate |
|---|---|---|---|---|---|---|---|
| dataset | forced_pipeline | 4.5 | 3.0 | 0 | 0 | 0 | 0.00 |
| dataset | none | 7.2 | 7.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | forced_pipeline | 7.0 | 4.0 | 0 | 0 | 0 | 0.00 |
| long-horizon | none | 12.0 | 9.2 | 0 | 0 | 4 | 0.00 |
| tool-use | forced_pipeline | 4.0 | 3.0 | 0 | 0 | 0 | 0.00 |
| tool-use | none | 13.5 | 12.5 | 2 | 0 | 0 | 0.00 |

## Notes

End-to-end through the real kernel. v0/v1 scripted-vs-real model is recorded via the provider/config; success/correctness are ground-truth checked; confidence intervals are Wilson (rates) and normal approx (latency/cost, n>=2). See docs/benchmark-spec.md and docs/experiments/.

benchmark report written to experiments/results/e8-routing-deepseek.json
